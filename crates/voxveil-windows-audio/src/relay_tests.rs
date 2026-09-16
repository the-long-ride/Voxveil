use super::*;
use super::support::{
    decide_backend, parse_loaded_instances, parse_loaded_instances_required, profile_control_value,
    should_fail_closed_after_probe, should_sync_apo_after_relay,
};
use crate::device::{RelayReadiness, WindowsInterceptionKind};

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
fn backend_defaults_to_music_preservation() {
    let backend = WindowsAudioBackend::new();
    assert_eq!(backend.classic_suppression_profile, ClassicSuppressionProfile::MusicPreservation);
}

#[test]
fn profile_control_values_match_the_shared_wire_contract() {
    assert_eq!(profile_control_value(ClassicSuppressionProfile::MusicPreservation), "music-preservation");
    assert_eq!(profile_control_value(ClassicSuppressionProfile::Balanced), "balanced");
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
    assert_eq!(decision.route.interception, Some(WindowsInterceptionKind::VoxveilCableRelay));
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
    assert_eq!(decision.route.interception, Some(WindowsInterceptionKind::VbCableRelay));
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
    assert_eq!(parse_loaded_instances("enabled=1 vocal=40 heartbeat=88 loaded=2"), Some(2));
}

#[test]
fn parses_loaded_count_from_capx_status() {
    assert_eq!(
        parse_loaded_instances("enabled=1 vocal=30 heartbeat=99 loaded=1 system-effect=1 capx=2"),
        Some(1)
    );
}

#[test]
fn discovery_only_capx_status_does_not_report_a_loaded_instance() {
    assert_eq!(
        parse_loaded_instances("enabled=0 vocal=100 heartbeat=0 loaded=0 system-effect=1 capx=1"),
        Some(0)
    );
}

#[test]
fn required_loaded_parser_rejects_malformed_status() {
    assert_eq!(parse_loaded_instances_required("enabled=1 loaded=2 capx=1"), Ok(2));
    assert!(parse_loaded_instances_required("enabled=1 capx=1").is_err());
    assert!(parse_loaded_instances_required("loaded=not-a-number").is_err());
}

#[test]
fn rejects_status_without_load_marker() {
    assert_eq!(parse_loaded_instances("enabled=1 vocal=40 capx=2"), None);
}
