#[cfg(target_os = "windows")]
use voxveil_windows_audio::{SystemAudioEndpoint, SystemAudioEndpointStatus};

#[cfg(target_os = "windows")]
fn select_installable_endpoint(
    _endpoints: Vec<SystemAudioEndpoint>,
    _endpoint_id: &str,
) -> Result<SystemAudioEndpoint, String> {
    Err("endpoint install lookup is not implemented".into())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    fn endpoint(id: &str, status: SystemAudioEndpointStatus) -> SystemAudioEndpoint {
        SystemAudioEndpoint {
            endpoint_id: id.into(),
            display_name: "Speakers".into(),
            adapter_name: Some("Example Audio".into()),
            is_default: true,
            pnp_instance_id: Some("HDAUDIO\\EXAMPLE".into()),
            hardware_ids: vec!["HDAUDIO\\EXAMPLE".into()],
            driver_inf: Some("oem42.inf".into()),
            topology_reference: Some("Topology".into()),
            status,
            detail: None,
        }
    }

    #[test]
    fn install_lookup_uses_only_endpoint_id() {
        let selected = select_installable_endpoint(
            vec![endpoint("endpoint-a", SystemAudioEndpointStatus::Installable)],
            "endpoint-a",
        )
        .unwrap();
        assert_eq!(selected.endpoint_id, "endpoint-a");
        assert_eq!(selected.hardware_ids[0], "HDAUDIO\\EXAMPLE");
    }

    #[test]
    fn ambiguous_endpoint_cannot_be_installed() {
        let error = select_installable_endpoint(
            vec![endpoint("endpoint-a", SystemAudioEndpointStatus::Ambiguous)],
            "endpoint-a",
        )
        .unwrap_err();
        assert!(error.contains("not installable"));
    }

    #[test]
    fn unknown_endpoint_is_rejected() {
        let error = select_installable_endpoint(Vec::new(), "missing").unwrap_err();
        assert!(error.contains("no longer available"));
    }
}
