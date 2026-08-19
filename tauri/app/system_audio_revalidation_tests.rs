#[cfg(target_os = "windows")]
use super::system_audio::validate_revalidated_binding;
#[cfg(target_os = "windows")]
use voxveil_windows_audio::{SystemAudioEndpoint, SystemAudioEndpointStatus};

#[cfg(target_os = "windows")]
fn endpoint(
    pnp_instance_id: &str,
    topology_reference: &str,
    status: SystemAudioEndpointStatus,
) -> SystemAudioEndpoint {
    SystemAudioEndpoint {
        endpoint_id: "endpoint-a".into(),
        display_name: "Speakers".into(),
        adapter_name: Some("Example Audio".into()),
        is_default: true,
        pnp_instance_id: Some(pnp_instance_id.into()),
        hardware_ids: vec!["HDAUDIO\\EXAMPLE".into()],
        driver_inf: Some("oem42.inf".into()),
        topology_reference: Some(topology_reference.into()),
        status,
        detail: None,
    }
}

#[cfg(target_os = "windows")]
#[test]
fn changed_binding_is_rejected_before_elevation() {
    let selected = endpoint(
        "HDAUDIO\\DEVICE_A",
        "PrimaryLineOutTopo",
        SystemAudioEndpointStatus::Installable,
    );
    let current = endpoint(
        "HDAUDIO\\DEVICE_B",
        "PrimaryLineOutTopo",
        SystemAudioEndpointStatus::Installable,
    );
    let error = validate_revalidated_binding(&selected, &current).unwrap_err();
    assert!(error.contains("changed"));
}

#[cfg(target_os = "windows")]
#[test]
fn binding_that_becomes_ambiguous_is_rejected_before_elevation() {
    let selected = endpoint(
        "HDAUDIO\\DEVICE_A",
        "PrimaryLineOutTopo",
        SystemAudioEndpointStatus::Installable,
    );
    let current = endpoint(
        "HDAUDIO\\DEVICE_A",
        "PrimaryLineOutTopo",
        SystemAudioEndpointStatus::Ambiguous,
    );
    let error = validate_revalidated_binding(&selected, &current).unwrap_err();
    assert!(error.contains("changed"));
}
