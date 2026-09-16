use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use wasapi::{DeviceEnumerator, Direction};

use crate::apo_route::{apo_covers_default_endpoint, load_installed_apo_endpoint};
use crate::device::{
    BackendProbe, EndpointDescriptor, RelayReadiness, WindowsAudioRoute, WindowsInterceptionKind,
};
use crate::discovery::{SystemAudioEndpoint, SystemAudioEndpointStatus, enrich_endpoints};
use crate::relay_engine::{RelayHandle, RelayRuntimeState, RelaySpec};
use crate::route::select_physical_output;
use crate::virtual_endpoint::{
    VirtualEndpointKind, classify_virtual_endpoint, find_preferred_virtual_endpoint,
};

pub struct WindowsAudioBackend {
    enabled: bool,
    vocal_level: u8,
    relay: Option<RelayHandle>,
    preferred_physical_output_id: Option<String>,
}

impl WindowsAudioBackend {
    pub fn new() -> Self {
        Self {
            enabled: false,
            vocal_level: 100,
            relay: None,
            preferred_physical_output_id: None,
        }
    }

    pub fn probe(&mut self) -> BackendProbe {
        let endpoints = match enumerate_render_blocking() {
            Ok(endpoints) => endpoints,
            Err(error) => {
                self.disable_processing_best_effort();
                return fault_probe(None, error);
            }
        };
        let (loaded_instances, apo_covers_default) = match query_apo_coverage(&endpoints) {
            Ok(value) => value,
            Err(error) => {
                self.disable_processing_best_effort();
                return fault_probe(
                    None,
                    format!("failed to query Voxveil APO status: {error}"),
                );
            }
        };
        let selected = find_preferred_virtual_endpoint(&endpoints);
        let virtual_id = selected
            .map(|(_, endpoint)| endpoint.id.as_str())
            .unwrap_or("");
        let physical = select_physical_output(
            &endpoints,
            virtual_id,
            self.preferred_physical_output_id.as_deref(),
        );

        if apo_covers_default {
            let relay_was_active = self.relay.is_some();
            if let Some(mut relay) = self.relay.take() {
                if let Err(error) = relay.stop() {
                    self.disable_processing_best_effort();
                    return fault_probe(
                        physical.map(|endpoint| endpoint.name.clone()),
                        format!("failed to stop Windows audio relay during APO handoff: {error}"),
                    );
                }
            }
            if should_sync_apo_after_relay(apo_covers_default, relay_was_active, self.enabled) {
                if let Err(error) = sync_apo_control(self.vocal_level, true) {
                    self.disable_processing_best_effort();
                    return fault_probe(
                        physical.map(|endpoint| endpoint.name.clone()),
                        format!("failed to transfer active processing state to Voxveil APO: {error}"),
                    );
                }
            }
            return decide_backend(true, selected, physical, None);
        }

        if loaded_instances > 0 && self.enabled {
            if let Err(error) = set_apo_enabled(false) {
                self.disable_processing_best_effort();
                return fault_probe(
                    physical.map(|endpoint| endpoint.name.clone()),
                    format!("failed to disable a Voxveil APO loaded on another endpoint: {error}"),
                );
            }
        }

        let relay_state = self.relay.as_ref().map(RelayHandle::state);
        let decision = decide_backend(false, selected, physical, relay_state.as_ref());
        if should_fail_closed_after_probe(decision.readiness) {
            self.disable_processing_best_effort();
        }
        decision
    }

    pub fn set_enabled(&mut self, enabled: bool, vocal_level: u8) -> Result<BackendProbe, String> {
        self.vocal_level = vocal_level.min(100);

        if !enabled {
            self.enabled = false;
            if let Some(mut relay) = self.relay.take() {
                relay.stop()?;
            }
            if control_executable().is_some() {
                set_apo_enabled(false)?;
            }
            return Ok(self.probe());
        }

        let endpoints = enumerate_render_blocking()?;
        let (loaded_instances, apo_covers_default) = match query_apo_coverage(&endpoints) {
            Ok(value) => value,
            Err(error) => {
                self.disable_processing_best_effort();
                return Err(format!("failed to query Voxveil APO status: {error}"));
            }
        };
        if apo_covers_default {
            if let Some(mut relay) = self.relay.take() {
                relay.stop()?;
            }
            sync_apo_control(self.vocal_level, true)?;
            self.enabled = true;
            return Ok(self.probe());
        }

        if loaded_instances > 0 {
            set_apo_enabled(false).map_err(|error| {
                format!("failed to disable a Voxveil APO loaded on another endpoint: {error}")
            })?;
        }

        let (virtual_kind, source) = find_preferred_virtual_endpoint(&endpoints).ok_or_else(|| {
            "Install the standard VB-CABLE virtual audio device or a verified Voxveil virtual audio driver to enable system-wide processing"
                .to_string()
        })?;
        if !source.is_default {
            return Err(match virtual_kind {
                VirtualEndpointKind::VoxveilCable => {
                    "Set Voxveil Input as the Windows default output before enabling Voxveil".into()
                }
                VirtualEndpointKind::VbCable => {
                    "Set CABLE Input as the Windows default output before enabling Voxveil".into()
                }
            });
        }
        let physical = select_physical_output(
            &endpoints,
            &source.id,
            self.preferred_physical_output_id.as_deref(),
        )
        .ok_or_else(|| "No safe physical playback endpoint is available for the relay".to_string())?;

        if let Some(mut relay) = self.relay.take() {
            relay.stop()?;
        }
        let spec = RelaySpec {
            source_endpoint_id: source.id.clone(),
            physical_output_endpoint_id: physical.id.clone(),
        };
        let relay = RelayHandle::start_wasapi(spec, self.vocal_level)?;
        self.relay = Some(relay);

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let state = self
                .relay
                .as_ref()
                .map(RelayHandle::state)
                .unwrap_or(RelayRuntimeState::Stopped);
            match state {
                RelayRuntimeState::Running => {
                    self.enabled = true;
                    return Ok(self.probe());
                }
                RelayRuntimeState::Faulted(error) => {
                    self.relay.take();
                    self.enabled = false;
                    return Err(error);
                }
                RelayRuntimeState::Stopped => {
                    self.relay.take();
                    self.enabled = false;
                    return Err("Windows audio relay stopped during startup".into());
                }
                RelayRuntimeState::Starting if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                RelayRuntimeState::Starting => {
                    if let Some(mut relay) = self.relay.take() {
                        let _ = relay.stop();
                    }
                    self.enabled = false;
                    return Err("Windows audio relay did not become ready within 2 seconds".into());
                }
            }
        }
    }

    pub fn set_vocal_level(&mut self, value: u8) {
        self.vocal_level = value.min(100);
        if let Some(relay) = &self.relay {
            let _ = relay.set_vocal_level(self.vocal_level);
        }
        if let Some(control) = control_executable() {
            let percent = self.vocal_level.to_string();
            let _ = run_control(&control, &["vocal", percent.as_str()]);
        }
    }

    pub fn set_physical_output(
        &mut self,
        endpoint_id: Option<String>,
    ) -> Result<BackendProbe, String> {
        if let Some(endpoint_id) = endpoint_id.as_deref() {
            let endpoints = enumerate_render_blocking()?;
            let endpoint = endpoints
                .iter()
                .find(|endpoint| endpoint.id == endpoint_id)
                .ok_or_else(|| "The selected physical playback endpoint is no longer available".to_string())?;
            if classify_virtual_endpoint(endpoint).is_some() {
                return Err("A virtual interception endpoint cannot be used as physical output".into());
            }
        }

        if let Some(mut relay) = self.relay.take() {
            self.enabled = false;
            relay.stop()?;
        }
        self.preferred_physical_output_id = endpoint_id;
        Ok(self.probe())
    }

    pub fn physical_outputs(&self) -> Vec<EndpointDescriptor> {
        enumerate_render_blocking()
            .map(|items| {
                items
                    .into_iter()
                    .filter(|item| classify_virtual_endpoint(item).is_none())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn system_audio_endpoints(&self) -> Result<Vec<SystemAudioEndpoint>, String> {
        let endpoints = enumerate_render_blocking()?;
        let (_, apo_covers_default) = query_apo_coverage(&endpoints)?;
        let directory = system_audio_directory();
        let helper = directory.join("discover-system-audio-endpoints.ps1");
        let mut enriched = enrich_endpoints(endpoints, &helper, &directory)?;

        if apo_covers_default {
            if let Some(active) = enriched.iter_mut().find(|endpoint| endpoint.is_default) {
                active.status = SystemAudioEndpointStatus::Ready;
                active.detail = None;
            }
        }
        Ok(enriched)
    }

    fn disable_processing_best_effort(&mut self) {
        self.enabled = false;
        if let Some(mut relay) = self.relay.take() {
            let _ = relay.stop();
        }
        if let Some(control) = control_executable() {
            let _ = run_control(&control, &["enabled", "0"]);
        }
    }
}

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

fn should_sync_apo_after_relay(apo_loaded: bool, relay_was_active: bool, enabled: bool) -> bool {
    apo_loaded && relay_was_active && enabled
}

fn should_fail_closed_after_probe(readiness: RelayReadiness) -> bool {
    readiness != RelayReadiness::Ready
}

fn decide_backend(
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

fn fault_probe(physical_output: Option<String>, error: String) -> BackendProbe {
    BackendProbe {
        readiness: RelayReadiness::Faulted,
        route: WindowsAudioRoute {
            physical_output_display_name: physical_output,
            ..WindowsAudioRoute::default()
        },
        detail: Some(error),
    }
}

fn set_apo_enabled(enabled: bool) -> Result<(), String> {
    let control = control_executable().ok_or_else(|| {
        "Voxveil APO reports a loaded instance but its control component is unavailable".to_string()
    })?;
    run_control(&control, &["enabled", if enabled { "1" } else { "0" }])?;
    Ok(())
}

fn sync_apo_control(vocal_level: u8, enabled: bool) -> Result<(), String> {
    let control = control_executable().ok_or_else(|| {
        "Voxveil APO reports a loaded instance but its control component is unavailable".to_string()
    })?;
    let percent = vocal_level.min(100).to_string();
    let profile = match crate::profile::classic_suppression_profile() {
        voxveil_types::ClassicSuppressionProfile::MusicPreservation => "music-preservation",
        voxveil_types::ClassicSuppressionProfile::Balanced => "balanced",
    };
    run_control(&control, &["profile", profile])?;
    run_control(&control, &["vocal", percent.as_str()])?;
    run_control(&control, &["enabled", if enabled { "1" } else { "0" }])?;
    Ok(())
}

fn query_apo_coverage(endpoints: &[EndpointDescriptor]) -> Result<(u32, bool), String> {
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

fn loaded_apo_instances() -> Result<u32, String> {
    let Some(control) = control_executable() else {
        return Ok(0);
    };
    let status = run_control(&control, &["status"])?;
    parse_loaded_instances_required(&status)
}

fn system_audio_directory() -> PathBuf {
    if let Ok(path) = env::var("VOXVEIL_SYSTEM_AUDIO_DIR") {
        return PathBuf::from(path);
    }
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("system-audio")))
        .unwrap_or_else(|| PathBuf::from("system-audio"))
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

fn parse_loaded_instances_required(status: &str) -> Result<u32, String> {
    parse_loaded_instances(status).ok_or_else(|| {
        "Voxveil APO control loaded=<count> status field is missing or invalid".to_string()
    })
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
            interface_name: device.get_interface_friendlyname().ok(),
            description: device.get_description().ok(),
            id,
            name,
        });
    }
    Ok(endpoints)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn physical(id: &str, is_default: bool) -> EndpointDescriptor {
        EndpointDescriptor {
            id: id.into(),
            name: "Speakers".into(),
            interface_name: Some("Physical Audio".into()),
            description: Some("Speakers".into()),
            is_default,
        }
    }

    fn vb_cable(is_default: bool) -> EndpointDescriptor {
        EndpointDescriptor {
            id: "cable".into(),
            name: "CABLE Input (VB-Audio Virtual Cable)".into(),
            interface_name: Some("VB-Audio Virtual Cable".into()),
            description: Some("CABLE Input".into()),
            is_default,
        }
    }

    fn voxveil_cable(is_default: bool) -> EndpointDescriptor {
        EndpointDescriptor {
            id: "voxveil".into(),
            name: "Voxveil Input".into(),
            interface_name: Some("Voxveil Virtual Audio".into()),
            description: Some("Voxveil Input".into()),
            is_default,
        }
    }

    #[test]
    fn loaded_apo_has_priority_over_virtual_relay() {
        let cable = voxveil_cable(true);
        let physical = physical("speakers", false);
        let decision = decide_backend(
            true,
            Some((VirtualEndpointKind::VoxveilCable, &cable)),
            Some(&physical),
            None,
        );
        assert_eq!(decision.route.interception, Some(WindowsInterceptionKind::Apo));
        assert_eq!(decision.readiness, RelayReadiness::Ready);
    }

    #[test]
    fn missing_virtual_endpoint_requires_component_when_apo_not_loaded() {
        let physical = physical("speakers", true);
        let decision = decide_backend(false, None, Some(&physical), None);
        assert_eq!(decision.readiness, RelayReadiness::ComponentRequired);
    }

    #[test]
    fn installed_vb_cable_must_be_default_for_all_output_capture() {
        let cable = vb_cable(false);
        let physical = physical("speakers", true);
        let decision = decide_backend(
            false,
            Some((VirtualEndpointKind::VbCable, &cable)),
            Some(&physical),
            None,
        );
        assert_eq!(decision.readiness, RelayReadiness::RoutingRequired);
        assert!(decision.detail.unwrap().contains("CABLE Input"));
    }

    #[test]
    fn voxveil_route_reports_voxveil_interception_kind() {
        let cable = voxveil_cable(true);
        let physical = physical("speakers", false);
        let decision = decide_backend(
            false,
            Some((VirtualEndpointKind::VoxveilCable, &cable)),
            Some(&physical),
            Some(&RelayRuntimeState::Running),
        );
        assert_eq!(decision.readiness, RelayReadiness::Ready);
        assert_eq!(
            decision.route.interception,
            Some(WindowsInterceptionKind::VoxveilCableRelay)
        );
    }

    #[test]
    fn vb_cable_route_is_ready_only_when_worker_is_running() {
        let cable = vb_cable(true);
        let physical = physical("speakers", false);
        let decision = decide_backend(
            false,
            Some((VirtualEndpointKind::VbCable, &cable)),
            Some(&physical),
            Some(&RelayRuntimeState::Running),
        );
        assert_eq!(decision.readiness, RelayReadiness::Ready);
        assert_eq!(
            decision.route.interception,
            Some(WindowsInterceptionKind::VbCableRelay)
        );
    }

    #[test]
    fn relay_fault_is_reported() {
        let cable = vb_cable(true);
        let physical = physical("speakers", false);
        let decision = decide_backend(
            false,
            Some((VirtualEndpointKind::VbCable, &cable)),
            Some(&physical),
            Some(&RelayRuntimeState::Faulted("device removed".into())),
        );
        assert_eq!(decision.readiness, RelayReadiness::Faulted);
        assert_eq!(decision.detail.as_deref(), Some("device removed"));
    }

    #[test]
    fn active_relay_handoff_to_loaded_apo_requires_control_sync() {
        assert!(should_sync_apo_after_relay(true, true, true));
        assert!(!should_sync_apo_after_relay(false, true, true));
        assert!(!should_sync_apo_after_relay(true, false, true));
        assert!(!should_sync_apo_after_relay(true, true, false));
    }

    #[test]
    fn only_ready_probe_decisions_may_keep_processing_active() {
        assert!(!should_fail_closed_after_probe(RelayReadiness::Ready));
        assert!(should_fail_closed_after_probe(RelayReadiness::RoutingRequired));
        assert!(should_fail_closed_after_probe(RelayReadiness::ComponentRequired));
        assert!(should_fail_closed_after_probe(RelayReadiness::Faulted));
        assert!(should_fail_closed_after_probe(RelayReadiness::Unsupported));
    }

    #[test]
    fn parses_loaded_instance_count() {
        assert_eq!(
            parse_loaded_instances("enabled=1 vocal=40 heartbeat=88 loaded=2"),
            Some(2)
        );
    }

    #[test]
    fn parses_loaded_count_from_capx_status() {
        assert_eq!(
            parse_loaded_instances(
                "enabled=1 vocal=30 heartbeat=99 loaded=1 system-effect=1 capx=2"
            ),
            Some(1)
        );
    }

    #[test]
    fn discovery_only_capx_status_does_not_report_a_loaded_instance() {
        assert_eq!(
            parse_loaded_instances(
                "enabled=0 vocal=100 heartbeat=0 loaded=0 system-effect=1 capx=1"
            ),
            Some(0)
        );
    }

    #[test]
    fn required_loaded_parser_rejects_malformed_status() {
        assert_eq!(
            parse_loaded_instances_required("enabled=1 loaded=2 capx=1"),
            Ok(2)
        );
        assert!(parse_loaded_instances_required("enabled=1 capx=1").is_err());
        assert!(parse_loaded_instances_required("loaded=not-a-number").is_err());
    }

    #[test]
    fn rejects_status_without_load_marker() {
        assert_eq!(parse_loaded_instances("enabled=1 vocal=40 capx=2"), None);
    }
}
