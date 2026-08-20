#[cfg(target_os = "windows")]
use super::system_audio::validate_revalidated_binding;
#[cfg(target_os = "windows")]
use voxveil_windows_audio::{SystemAudioEndpoint, SystemAudioEndpointStatus};

#[cfg(target_os = "windows")]
fn endpoint(
    binding_pnp_instance_id: &str,
    pnp_instance_id: &str,
    topology_interface_path: &str,
    audio_interface_path: &str,
    status: SystemAudioEndpointStatus,
) -> SystemAudioEndpoint {
    SystemAudioEndpoint {
        endpoint_id: "endpoint-a".into(),
        display_name: "Speakers".into(),
        adapter_name: Some("Example Audio".into()),
        is_default: true,
        binding_pnp_instance_id: Some(binding_pnp_instance_id.into()),
        pnp_instance_id: Some(pnp_instance_id.into()),
        hardware_ids: vec!["HDAUDIO\\EXAMPLE".into()],
        driver_inf: Some("oem42.inf".into()),
        topology_interface_path: Some(topology_interface_path.into()),
        audio_interface_path: Some(audio_interface_path.into()),
        topology_reference: None,
        status,
        detail: None,
    }
}

#[cfg(target_os = "windows")]
#[test]
fn changed_metadata_device_is_rejected_before_elevation() {
    let selected = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_A",
        "topology-a",
        "audio-a",
        SystemAudioEndpointStatus::Installable,
    );
    let current = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_B",
        "topology-a",
        "audio-a",
        SystemAudioEndpointStatus::Installable,
    );
    let error = validate_revalidated_binding(&selected, &current).unwrap_err();
    assert!(error.contains("changed"));
}

#[cfg(target_os = "windows")]
#[test]
fn changed_topology_binding_is_rejected_before_elevation() {
    let selected = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_A",
        "topology-a",
        "audio-a",
        SystemAudioEndpointStatus::Installable,
    );
    let current = endpoint(
        "HDAUDIO\\BINDING_B",
        "HDAUDIO\\DEVICE_A",
        "topology-a",
        "audio-a",
        SystemAudioEndpointStatus::Installable,
    );
    let error = validate_revalidated_binding(&selected, &current).unwrap_err();
    assert!(error.contains("changed"));
}

#[cfg(target_os = "windows")]
#[test]
fn changed_topology_interface_path_is_rejected_before_elevation() {
    let selected = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_A",
        "topology-a",
        "audio-a",
        SystemAudioEndpointStatus::Installable,
    );
    let current = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_A",
        "topology-b",
        "audio-a",
        SystemAudioEndpointStatus::Installable,
    );
    let error = validate_revalidated_binding(&selected, &current).unwrap_err();
    assert!(error.contains("changed"));
}

#[cfg(target_os = "windows")]
#[test]
fn changed_audio_interface_path_is_rejected_before_elevation() {
    let selected = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_A",
        "topology-a",
        "audio-a",
        SystemAudioEndpointStatus::Installable,
    );
    let current = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_A",
        "topology-a",
        "audio-b",
        SystemAudioEndpointStatus::Installable,
    );
    let error = validate_revalidated_binding(&selected, &current).unwrap_err();
    assert!(error.contains("changed"));
}

#[cfg(target_os = "windows")]
#[test]
fn binding_that_becomes_ambiguous_is_rejected_before_elevation() {
    let selected = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_A",
        "topology-a",
        "audio-a",
        SystemAudioEndpointStatus::Installable,
    );
    let current = endpoint(
        "HDAUDIO\\BINDING_A",
        "HDAUDIO\\DEVICE_A",
        "topology-a",
        "audio-a",
        SystemAudioEndpointStatus::Ambiguous,
    );
    let error = validate_revalidated_binding(&selected, &current).unwrap_err();
    assert!(error.contains("changed"));
}
