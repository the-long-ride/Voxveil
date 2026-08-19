use std::env;
use std::path::PathBuf;
use std::process::Command;

use wasapi::{DeviceEnumerator, Direction};

use crate::device::{BackendProbe, EndpointDescriptor, RelayReadiness, component_probe};

pub struct WindowsAudioBackend {
    vocal_level: u8,
}

impl WindowsAudioBackend {
    pub fn new() -> Self {
        Self { vocal_level: 100 }
    }

    pub fn probe(&mut self) -> BackendProbe {
        let physical_output = default_render_name().ok().flatten();
        let Some(control) = control_executable() else {
            return component_probe(false, 0, physical_output);
        };

        match run_control(&control, &["status"]) {
            Ok(output) => match parse_loaded_instances(&output) {
                Some(loaded) => component_probe(true, loaded, physical_output),
                None => fault_probe(
                    physical_output,
                    "Voxveil control status did not contain a loaded instance count".into(),
                ),
            },
            Err(error) => fault_probe(physical_output, error),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool, vocal_level: u8) -> Result<BackendProbe, String> {
        self.vocal_level = vocal_level.min(100);
        let control = control_executable().ok_or_else(|| {
            "Voxveil system-audio control component is not installed beside the application"
                .to_string()
        })?;

        run_control(&control, &["vocal", &self.vocal_level.to_string()])?;
        run_control(&control, &["enabled", if enabled { "1" } else { "0" }])?;

        let probe = self.probe();
        if enabled && probe.readiness != RelayReadiness::Ready {
            return Err(probe
                .detail
                .clone()
                .unwrap_or_else(|| "Voxveil APO is not attached to the active render endpoint".into()));
        }
        Ok(probe)
    }

    pub fn set_vocal_level(&self, value: u8) {
        if let Some(control) = control_executable() {
            let percent = value.min(100).to_string();
            let _ = run_control(&control, &["vocal", &percent]);
        }
    }

    pub fn physical_outputs(&self) -> Vec<String> {
        enumerate_render_blocking()
            .map(|items| items.into_iter().map(|item| item.name).collect())
            .unwrap_or_default()
    }
}

fn fault_probe(physical_output: Option<String>, error: String) -> BackendProbe {
    BackendProbe {
        readiness: RelayReadiness::Faulted,
        physical_output,
        detail: Some(error),
    }
}

fn control_executable() -> Option<PathBuf> {
    if let Ok(path) = env::var("VOXVEIL_CONTROL_EXE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let directory = env::current_exe().ok()?.parent()?.to_path_buf();
    [
        directory.join("voxveil-control.exe"),
        directory.join("system-audio").join("voxveil-control.exe"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn run_control(control: &PathBuf, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(control);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = command
        .output()
        .map_err(|error| format!("failed to run {}: {error}", control.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{} exited with {}", control.display(), output.status)
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_loaded_instances(status: &str) -> Option<u32> {
    status
        .split_whitespace()
        .find_map(|part| part.strip_prefix("loaded=")?.parse().ok())
}

fn default_render_name() -> Result<Option<String>, String> {
    let endpoints = enumerate_render_blocking()?;
    Ok(endpoints
        .into_iter()
        .find(|endpoint| endpoint.is_default)
        .map(|endpoint| endpoint.name))
}

fn enumerate_render_blocking() -> Result<Vec<EndpointDescriptor>, String> {
    std::thread::spawn(move || {
        wasapi::initialize_mta()
            .ok()
            .map_err(|error| error.to_string())?;
        let result = enumerate_render_inner();
        wasapi::deinitialize();
        result
    })
    .join()
    .map_err(|_| "Windows endpoint enumeration panicked".to_string())?
}

fn enumerate_render_inner() -> Result<Vec<EndpointDescriptor>, String> {
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let default_id = enumerator
        .get_default_device(&Direction::Render)
        .and_then(|device| device.get_id())
        .unwrap_or_default();
    let collection = enumerator
        .get_device_collection(&Direction::Render)
        .map_err(|error| error.to_string())?;
    let mut endpoints = Vec::new();
    for device in &collection {
        let device = device.map_err(|error| error.to_string())?;
        let id = device.get_id().map_err(|error| error.to_string())?;
        let name = device
            .get_friendlyname()
            .map_err(|error| error.to_string())?;
        endpoints.push(EndpointDescriptor {
            is_default: id == default_id,
            id,
            name,
        });
    }
    Ok(endpoints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_loaded_instance_count() {
        assert_eq!(
            parse_loaded_instances("enabled=1 vocal=40 heartbeat=88 loaded=2"),
            Some(2)
        );
    }

    #[test]
    fn rejects_status_without_load_marker() {
        assert_eq!(parse_loaded_instances("enabled=1 vocal=40"), None);
    }
}
