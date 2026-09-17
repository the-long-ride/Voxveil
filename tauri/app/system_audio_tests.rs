use super::*;

fn endpoint(id: &str, status: SystemAudioEndpointStatus) -> SystemAudioEndpoint {
    SystemAudioEndpoint {
        endpoint_id: id.into(),
        display_name: "Speakers".into(),
        adapter_name: Some("Example Audio".into()),
        is_default: true,
        binding_pnp_instance_id: Some("HDAUDIO\\EXAMPLE".into()),
        pnp_instance_id: Some("HDAUDIO\\EXAMPLE".into()),
        hardware_ids: vec!["HDAUDIO\\EXAMPLE".into()],
        driver_inf: Some("oem42.inf".into()),
        topology_interface_path: Some("\\\\?\\topology-example".into()),
        audio_interface_path: Some("\\\\?\\audio-example".into()),
        topology_reference: None,
        status,
        detail: None,
    }
}

#[test]
fn install_lookup_uses_only_endpoint_id() {
    let selected = select_installable_endpoint(
        vec![endpoint("endpoint-a", SystemAudioEndpointStatus::Installable)],
        "endpoint-a",
    ).unwrap();
    assert_eq!(selected.endpoint_id, "endpoint-a");
    assert_eq!(selected.hardware_ids[0], "HDAUDIO\\EXAMPLE");
}

#[test]
fn runtime_binding_does_not_require_topology_reference() {
    let selected = select_installable_endpoint(
        vec![endpoint("endpoint-a", SystemAudioEndpointStatus::Installable)],
        "endpoint-a",
    ).unwrap();
    assert!(selected.topology_reference.is_none());
    assert!(has_runtime_interface_binding(&selected));
}

#[test]
fn ambiguous_endpoint_cannot_be_installed() {
    let error = select_installable_endpoint(
        vec![endpoint("endpoint-a", SystemAudioEndpointStatus::Ambiguous)],
        "endpoint-a",
    ).unwrap_err();
    assert!(error.contains("not installable"));
}

#[test]
fn unknown_endpoint_is_rejected() {
    let error = select_installable_endpoint(Vec::new(), "missing").unwrap_err();
    assert!(error.contains("no longer available"));
}

#[test]
fn resolves_installer_beside_packaged_executable() {
    let executable = PathBuf::from("bundle").join("voxveil.exe");
    assert_eq!(
        system_audio_installer_path(&executable),
        Ok(PathBuf::from("bundle").join("system-audio").join("install-system-audio-component.ps1"))
    );
}

#[test]
fn escapes_apostrophes_for_powershell_single_quoted_strings() {
    assert_eq!(powershell_single_quoted("C:\\User's Files\\setup.ps1"), "C:\\User''s Files\\setup.ps1");
}

#[test]
fn maps_windows_reboot_required_exit_without_treating_it_as_failure() {
    assert_eq!(installer_launch_outcome(Some(0)), Ok(InstallerLaunchOutcome::Completed));
    assert_eq!(
        installer_launch_outcome(Some(3010)),
        Ok(InstallerLaunchOutcome::RebootRequired)
    );
    assert!(installer_launch_outcome(Some(1)).is_err());
    assert!(installer_launch_outcome(None).is_err());
}
