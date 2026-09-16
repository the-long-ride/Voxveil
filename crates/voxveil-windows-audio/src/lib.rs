#![deny(unsafe_code)]

mod apo_route;
mod binding;
mod device;
mod device_interfaces;
mod discovery;
mod profile;
mod relay_engine;
mod route;
mod sample;
mod virtual_endpoint;

#[cfg(windows)]
mod relay;
#[cfg(windows)]
mod wasapi_relay;
#[cfg(windows)]
#[allow(unsafe_code)]
mod topology;

use voxveil_types::ClassicSuppressionProfile;

pub use device::{
    BackendProbe, EndpointDescriptor, RelayReadiness, WindowsAudioRoute, WindowsInterceptionKind,
};
pub use discovery::{SystemAudioEndpoint, SystemAudioEndpointStatus};
pub use sample::process_f32le_stereo;

#[cfg(windows)]
pub use relay::WindowsAudioBackend;

#[cfg(windows)]
fn control_executable_for_profile() -> Option<std::path::PathBuf> {
    if let Ok(path) = std::env::var("VOXVEIL_CONTROL_EXE") {
        let path = std::path::PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let directory = std::env::current_exe().ok()?.parent()?.to_path_buf();
    [
        directory.join("voxveil-control.exe"),
        directory.join("system-audio").join("voxveil-control.exe"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

#[cfg(windows)]
fn run_profile_control(control: &std::path::Path, args: &[&str]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;

    let output = std::process::Command::new(control)
        .args(args)
        .creation_flags(0x0800_0000)
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

#[cfg(windows)]
fn loaded_apo_from_status(status: &str) -> u32 {
    status
        .split_whitespace()
        .find_map(|part| part.strip_prefix("loaded=")?.parse().ok())
        .unwrap_or(0)
}

#[cfg(windows)]
fn sync_profile_to_loaded_apo(profile: ClassicSuppressionProfile) -> Result<(), String> {
    let Some(control) = control_executable_for_profile() else {
        return Ok(());
    };
    let status = run_profile_control(&control, &["status"])?;
    if loaded_apo_from_status(&status) == 0 {
        return Ok(());
    }
    let value = match profile {
        ClassicSuppressionProfile::MusicPreservation => "music-preservation",
        ClassicSuppressionProfile::Balanced => "balanced",
    };
    run_profile_control(&control, &["profile", value])?;
    Ok(())
}

#[cfg(windows)]
impl WindowsAudioBackend {
    pub fn set_classic_suppression_profile(
        &mut self,
        profile: ClassicSuppressionProfile,
    ) -> Result<(), String> {
        sync_profile_to_loaded_apo(profile)?;
        profile::set_classic_suppression_profile(profile);
        Ok(())
    }
}

#[cfg(not(windows))]
pub struct WindowsAudioBackend;

#[cfg(not(windows))]
impl WindowsAudioBackend {
    pub fn new() -> Self {
        Self
    }
    pub fn probe(&self) -> BackendProbe {
        BackendProbe::unsupported()
    }
    pub fn set_enabled(
        &mut self,
        _enabled: bool,
        _vocal_level: u8,
    ) -> Result<BackendProbe, String> {
        Err("Windows audio relay is unavailable on this platform".into())
    }
    pub fn set_vocal_level(&self, _value: u8) {}
    pub fn set_classic_suppression_profile(
        &mut self,
        _profile: ClassicSuppressionProfile,
    ) -> Result<(), String> {
        Ok(())
    }
    pub fn physical_outputs(&self) -> Vec<String> {
        Vec::new()
    }
    pub fn system_audio_endpoints(&self) -> Result<Vec<SystemAudioEndpoint>, String> {
        Ok(Vec::new())
    }
}

#[cfg(all(test, windows))]
mod profile_control_tests {
    use super::*;

    #[test]
    fn loaded_apo_status_parser_is_fail_safe() {
        assert_eq!(loaded_apo_from_status("enabled=1 loaded=2 capx=1"), 2);
        assert_eq!(loaded_apo_from_status("enabled=1 capx=1"), 0);
        assert_eq!(loaded_apo_from_status("loaded=invalid"), 0);
    }
}
