#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemAudioEndpointStatus {
    Ready,
    Installable,
    ComponentRequired,
    Ambiguous,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemAudioEndpoint {
    pub endpoint_id: String,
    pub display_name: String,
    pub adapter_name: Option<String>,
    pub is_default: bool,
    pub binding_pnp_instance_id: Option<String>,
    pub pnp_instance_id: Option<String>,
    pub hardware_ids: Vec<String>,
    pub driver_inf: Option<String>,
    pub topology_interface_path: Option<String>,
    pub audio_interface_path: Option<String>,
    pub topology_reference: Option<String>,
    pub status: SystemAudioEndpointStatus,
    pub detail: Option<String>,
}

pub(crate) fn classify_binding(
    pnp_resolved: bool,
    topology_candidates: &[String],
    package_available: bool,
) -> SystemAudioEndpointStatus {
    if !pnp_resolved {
        return SystemAudioEndpointStatus::Unsupported;
    }
    match topology_candidates.len() {
        0 => SystemAudioEndpointStatus::Unsupported,
        1 if package_available => SystemAudioEndpointStatus::Installable,
        1 => SystemAudioEndpointStatus::ComponentRequired,
        _ => SystemAudioEndpointStatus::Ambiguous,
    }
}

pub(crate) fn extension_inf_matches_hardware(text: &str, hardware_ids: &[String]) -> bool {
    if hardware_ids.is_empty() {
        return false;
    }
    let text = text.to_ascii_lowercase();
    hardware_ids
        .iter()
        .any(|hardware_id| text.contains(&hardware_id.to_ascii_lowercase()))
}

pub(crate) fn capx_extension_inf_matches(
    text: &str,
    hardware_ids: &[String],
    topology_reference: Option<&str>,
) -> bool {
    const CAPX_CONTEXT_GUID: &str = "63e268ce-4cbc-48e0-beb6-55103316f477";
    if !extension_inf_matches_hardware(text, hardware_ids) {
        return false;
    }
    let text = text.to_ascii_lowercase();
    if !text.contains(CAPX_CONTEXT_GUID) || !text.contains("voxveil_apo_context") {
        return false;
    }
    if topology_reference.is_some_and(|reference| {
        reference.is_empty() || !text.contains(&reference.to_ascii_lowercase())
    }) {
        return false;
    }

    let mut has_add_interface = false;
    let mut has_context_association = false;
    let mut has_legacy_root_association = false;
    for line in text.lines() {
        let directive: String = line
            .split(';')
            .next()
            .unwrap_or_default()
            .chars()
            .filter(|value| !value.is_ascii_whitespace())
            .collect();
        if directive.starts_with("addinterface=") {
            has_add_interface = true;
        }
        if directive.starts_with(r"hkr,fx\0\%voxveil_apo_context%,%pkey_fx_association%") {
            has_context_association = true;
        }
        if directive.starts_with(r"hkr,fx\0,%pkey_fx_association%") {
            has_legacy_root_association = true;
        }
    }
    has_add_interface && has_context_association && !has_legacy_root_association
}

#[cfg(test)]
pub(crate) fn extension_inf_matches(
    text: &str,
    hardware_ids: &[String],
    topology_reference: &str,
) -> bool {
    if topology_reference.is_empty() {
        return false;
    }
    extension_inf_matches_hardware(text, hardware_ids)
        && text
            .to_ascii_lowercase()
            .contains(&topology_reference.to_ascii_lowercase())
}

#[cfg(windows)]
#[path = "discovery_windows.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::enrich_endpoints;

#[cfg(test)]
mod tests {
    use super::*;

    fn capx_inf(hardware_id: &str, reference: &str) -> String {
        format!(
            "Model={hardware_id}\nExtensionId={{1D81E93D-AB81-473B-9E5E-94FAE8D2377F}}\nVOXVEIL_APO_CONTEXT=\"{{63E268CE-4CBC-48E0-BEB6-55103316F477}}\"\nREFERENCE_STRING=\"{reference}\"\nHKR,FX\\0\\%VOXVEIL_APO_CONTEXT%,%PKEY_FX_Association%,,%KSNODETYPE_ANY%\nAddInterface=%KSCATEGORY_AUDIO%,%REFERENCE_STRING%,DeviceExtensions.I.APO\n"
        )
    }

    #[test]
    fn unique_topology_is_installable_when_package_is_available() {
        assert_eq!(classify_binding(true, &["Topology".into()], true), SystemAudioEndpointStatus::Installable);
    }

    #[test]
    fn multiple_topology_candidates_fail_closed() {
        assert_eq!(
            classify_binding(true, &["Topology".into(), "HeadphoneTopology".into()], true),
            SystemAudioEndpointStatus::Ambiguous
        );
    }

    #[test]
    fn missing_pnp_identity_is_unsupported() {
        assert_eq!(classify_binding(false, &["Topology".into()], true), SystemAudioEndpointStatus::Unsupported);
    }

    #[test]
    fn resolved_binding_without_installable_package_requires_component() {
        assert_eq!(classify_binding(true, &["Topology".into()], false), SystemAudioEndpointStatus::ComponentRequired);
    }

    #[test]
    fn signed_extension_must_match_hardware_and_topology_for_fallback() {
        let hardware_ids: Vec<String> = vec!["HDAUDIO\\FUNC_01&VEN_10EC".into()];
        let text = capx_inf(&hardware_ids[0], "PrimaryLineOutTopo");
        assert!(capx_extension_inf_matches(&text, &hardware_ids, Some("PrimaryLineOutTopo")));
        assert!(!capx_extension_inf_matches(&text, &hardware_ids, Some("HeadphoneTopo")));
        assert!(!capx_extension_inf_matches(&text, &["USB\\VID_1234".into()], Some("PrimaryLineOutTopo")));
    }

    #[test]
    fn runtime_binding_accepts_signed_capx_addinterface_package() {
        let hardware_ids: Vec<String> = vec!["HDAUDIO\\FUNC_01&VEN_10EC".into()];
        let text = capx_inf(&hardware_ids[0], "PrimaryLineOutTopo");
        assert!(capx_extension_inf_matches(&text, &hardware_ids, None));
    }

    #[test]
    fn capx_package_rejects_legacy_root_fx_association() {
        let hardware_ids: Vec<String> = vec!["HDAUDIO\\FUNC_01&VEN_10EC".into()];
        let mut text = capx_inf(&hardware_ids[0], "PrimaryLineOutTopo");
        text.push_str("HKR,FX\\0,%PKEY_FX_Association%,,%KSNODETYPE_ANY%\n");
        assert!(!capx_extension_inf_matches(&text, &hardware_ids, None));
    }

    #[test]
    fn capx_package_requires_signed_interface_binding() {
        let hardware_ids: Vec<String> = vec!["HDAUDIO\\FUNC_01&VEN_10EC".into()];
        let text = capx_inf(&hardware_ids[0], "PrimaryLineOutTopo")
            .lines()
            .filter(|line| !line.to_ascii_lowercase().starts_with("addinterface"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!capx_extension_inf_matches(&text, &hardware_ids, None));
    }

    #[test]
    fn legacy_non_capx_package_is_not_production_installable() {
        let text = "Model=HDAUDIO\\FUNC_01&VEN_10EC\n; runtime interface binding";
        let hardware_ids = vec!["HDAUDIO\\FUNC_01&VEN_10EC".into()];
        assert!(!capx_extension_inf_matches(&text, &hardware_ids, None));
    }

    #[test]
    fn legacy_topology_helper_still_matches_exact_reference_text() {
        let text = "HardwareId=HDAUDIO\\FUNC_01&VEN_10EC\nReference=PrimaryLineOutTopo";
        let hardware_ids = vec!["HDAUDIO\\FUNC_01&VEN_10EC".into()];
        assert!(extension_inf_matches(text, &hardware_ids, "PrimaryLineOutTopo"));
    }
}
