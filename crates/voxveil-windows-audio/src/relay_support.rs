use std::env;
use std::path::PathBuf;
use std::process::Command;

use voxveil_types::ClassicSuppressionProfile;

use crate::apo_route::{apo_covers_default_endpoint, load_installed_apo_endpoint};
use crate::device::{
    BackendProbe, EndpointDescriptor, RelayReadiness, WindowsAudioRoute, WindowsInterceptionKind,
};
use crate::relay_engine::RelayRuntimeState;
use crate::virtual_endpoint::VirtualEndpointKind;

fn interception_kind(kind: VirtualEndpointKind) -> WindowsInterceptionKind {
    match kind {
        VirtualEndpointKind::VoxveilCable => WindowsInterceptionKind::VoxveilCableRelay,
        VirtualEndpointKind::VbCable => WindowsInterceptionKind::VbCableRelay,
    }
}

fn route_label(kind: VirtualEndpointKind) -> &'static str {
    match kind {
        VirtualEndpointKind::VoxveilCable => "Voxveil virtual audio relay",
        VirtualEndpointKind::VbCable => "VB-CABLE relay",
    }
}

fn default_route_instruction(kind: VirtualEndpointKind) -> &'static str {
    match kind {
        VirtualEndpointKind::VoxveilCable => {
            "Set Voxveil Input as the Windows default output, then return to Voxveil"
        }
        VirtualEndpointKind::VbCable => {
            "Set CABLE Input as the Windows default output, then return to Voxveil"
        }
    }
}

pub(super) fn should_sync_apo_after_relay(
    apo_loaded: bool,
    relay_was_active: bool,
    enabled: bool,
) -> bool {
    apo_loaded && relay_was_active && enabled
}

pub(super) fn should_fail_closed_after_probe(readiness: RelayReadiness) -> bool {
    readiness != RelayReadiness::Ready
}

pub(super) fn decide_backend(
    apo_loaded: bool,
    selected: Option<(VirtualEndpointKind, &EndpointDescriptor)>,
    physical: Option<&EndpointDescriptor>,
    relay_state: Option<&RelayRuntimeState>,
) -> BackendProbe {
    let physical_route = |interception, source: Option<&EndpointDescriptor>| WindowsAudioRoute {
        interception,
        source_endpoint_id: source.map(|endpoint| endpoint.id.clone()),
        source_display_name: source.map(|endpoint| endpoint.name.clone()),
        physical_output_endpoint_id: physical.map(|endpoint| endpoint.id.clone()),
        physical_output_display_name: physical.map(|endpoint| endpoint.name.clone()),
    };

    if apo_loaded {
        return BackendProbe {
            readiness: RelayReadiness::Ready,
            route: physical_route(Some(WindowsInterceptionKind::Apo), None),
            detail: None,
        };
    }

    let Some((virtual_kind, source)) = selected else {
        return BackendProbe {
            readiness: RelayReadiness::ComponentRequired,
            route: physical_route(None, None),
            detail: Some(
                "Install the standard VB-CABLE virtual audio device or a verified Voxveil virtual audio driver to enable system-wide processing"
                    .into(),
            ),
        };
    };
    let route = physical_route(Some(interception_kind(virtual_kind)), Some(source));

    if !source.is_default {
        return BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            route,
            detail: Some(default_route_instruction(virtual_kind).into()),
        };
    }
    if physical.is_none() {
        return BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            route,
            detail: Some("Select a physical playback endpoint for Voxveil output".into()),
        };
    }

    match relay_state {
        Some(RelayRuntimeState::Running) => BackendProbe {
            readiness: RelayReadiness::Ready,
            route,
            detail: None,
        },
        Some(RelayRuntimeState::Faulted(error)) => BackendProbe {
            readiness: RelayReadiness::Faulted,
            route,
            detail: Some(error.clone()),
        },
        Some(RelayRuntimeState::Starting) => BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            route,
            detail: Some(format!("{} is starting", route_label(virtual_kind))),
        },
        Some(RelayRuntimeState::Stopped) | None => BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            route,
            detail: Some(format!("{} is configured and ready to start", route_label(virtual_kind))),
        },
    }
}

pub(super) fn fault_probe(physical_output: Option<String>, error: String) -> BackendProbe {
    BackendProbe {
        readiness: RelayReadiness::Faulted,
        route: WindowsAudioRoute {
            physical_output_display_name: physical_output,
            ..WindowsAudioRoute::default()
        },
        detail: Some(error),
    }
}

pub(super) fn set_apo_enabled(enabled: bool) -> Result<(), String> {
    let control = control_executable().ok_or_else(|| {
        "Voxveil APO reports a loaded instance but its control component is unavailable".to_string()
    })?;
    run_control(&control, &["enabled", if enabled { "1" } else { "0" }])?;
    Ok(())
}

pub(super) fn profile_control_value(profile: ClassicSuppressionProfile) -> &'static str {
    match profile {
        ClassicSuppressionProfile::MusicPreservation => "music-preservation",
        ClassicSuppressionProfile::Balanced => "balanced",
    }
}

pub(super) fn sync_apo_control(
    vocal_level: u8,
    profile: ClassicSuppressionProfile,
    enabled: bool,
) -> Result<(), String> {
    let control = control_executable().ok_or_else(|| {
        "Voxveil APO reports a loaded instance but its control component is unavailable".to_string()
    })?;
    let percent = vocal_level.min(100).to_string();
    run_control(&control, &["profile", profile_control_value(profile)])?;
    run_control(&control, &["vocal", percent.as_str()])?;
    run_control(&control, &["enabled", if enabled { "1" } else { "0" }])?;
    Ok(())
}

pub(super) fn query_apo_coverage(
    endpoints: &[EndpointDescriptor],
) -> Result<(u32, bool), String> {
    let loaded_instances = loaded_apo_instances()?;
    if loaded_instances == 0 {
        return Ok((0, false));
    }
    let installed_endpoint = load_installed_apo_endpoint(&system_audio_directory())?;
    let apo_covers_default = apo_covers_default_endpoint(
        loaded_instances,
        installed_endpoint.as_deref(),
        endpoints,
    );
    Ok((loaded_instances, apo_covers_default))
}

fn apo_install_state_exists() -> Result<bool, String> {
    let path = system_audio_directory().join("install-state.json");
    match std::fs::metadata(&path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "failed to inspect Voxveil APO install state at {}: {error}",
            path.display()
        )),
    }
}

pub(super) fn control_executable_for_installed_apo() -> Result<Option<PathBuf>, String> {
    let control = control_executable();
    if control.is_none() && apo_install_state_exists()? {
        return Err(
            "Voxveil APO install state exists but its control component is unavailable; load state cannot be verified"
                .to_string(),
        );
    }
    Ok(control)
}

fn loaded_apo_instances() -> Result<u32, String> {
    let Some(control) = control_executable_for_installed_apo()? else {
        return Ok(0);
    };
    let status = run_control(&control, &["status"])?;
    parse_loaded_instances_required(&status)
}

pub(super) fn system_audio_directory() -> PathBuf {
    if let Ok(path) = env::var("VOXVEIL_SYSTEM_AUDIO_DIR") {
        return PathBuf::from(path);
    }
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("system-audio")))
        .unwrap_or_else(|| PathBuf::from("system-audio"))
}

pub(super) fn control_executable() -> Option<PathBuf> {
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

pub(super) fn run_control(control: &PathBuf, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(control);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
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

pub(super) fn parse_loaded_instances(status: &str) -> Option<u32> {
    status
        .split_whitespace()
        .find_map(|part| part.strip_prefix("loaded=")?.parse().ok())
}

pub(super) fn parse_loaded_instances_required(status: &str) -> Result<u32, String> {
    parse_loaded_instances(status).ok_or_else(|| {
        "Voxveil APO control loaded=<count> status field is missing or invalid".to_string()
    })
}
