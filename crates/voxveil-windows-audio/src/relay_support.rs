use voxveil_types::{ClassicSuppressionProfile};

use super::control::{
    control_executable, loaded_apo_instances, run_control, system_audio_directory,
};
use crate::apo_route::{apo_covers_default_endpoint, load_installed_apo_endpoint};
use crate::device::{
    BackendProbe, EndpointDescriptor, RelayReadiness, WindowsAudioRoute,
};

pub(super) use crate::relay_decision::{
    decide_backend, decide_backend_with_policy, should_sync_apo_after_relay,
    should_fail_closed_after_probe,
};

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
        ClassicSuppressionProfile::Strong => "strong",
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

pub(super) fn query_apo_coverage(endpoints: &[EndpointDescriptor]) -> Result<(u32, bool), String> {
    let loaded_instances = loaded_apo_instances()?;
    if loaded_instances == 0 {
        return Ok((0, false));
    }
    let installed_endpoint = load_installed_apo_endpoint(&system_audio_directory())?;
    let apo_covers_default =
        apo_covers_default_endpoint(loaded_instances, installed_endpoint.as_deref(), endpoints);
    Ok((loaded_instances, apo_covers_default))
}
