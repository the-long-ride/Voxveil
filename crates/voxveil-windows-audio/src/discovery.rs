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
        if directive.starts_with(
            r"hkr,fx\0\%voxveil_apo_context%,%pkey_fx_association%",
        ) {
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
mod windows {
    use std::collections::HashMap;
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::{Command, Stdio};

    use serde::{Deserialize, Serialize};

    use super::{SystemAudioEndpoint, SystemAudioEndpointStatus, capx_extension_inf_matches};
    use crate::binding::{
        RuntimeBindingKind, classify_runtime_binding, fallback_device_matches_runtime,
    };
    use crate::device::EndpointDescriptor;
    use crate::device_interfaces::{
        CandidateSelection, TopologyCandidate, enumerate_topology_interfaces,
        select_topology_candidate,
    };
    use crate::topology::resolve_adapter_device_id;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const EXPECTED_EXTENSION_ID: &str = "1D81E93D-AB81-473B-9E5E-94FAE8D2377F";
    const EXPECTED_CAPX_CONTEXT: &str = "63E268CE-4CBC-48E0-BEB6-55103316F477";

    #[derive(Clone, Debug)]
    struct RuntimeResolution {
        binding_pnp_instance_id: Option<String>,
        topology_interface_path: Option<String>,
        audio_interface_path: Option<String>,
        kind: RuntimeBindingKind,
        alias_match: bool,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct InputEndpoint<'a> {
        endpoint_id: &'a str,
        display_name: &'a str,
        is_default: bool,
        runtime_device_id: Option<&'a str>,
        runtime_alias_match: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResolvedEndpoint {
        endpoint_id: String,
        binding_pnp_instance_id: Option<String>,
        adapter_name: Option<String>,
        pnp_instance_id: Option<String>,
        #[serde(default)]
        hardware_ids: Vec<String>,
        driver_inf: Option<String>,
        #[serde(default)]
        topology_references: Vec<String>,
        detail: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ApoVerification {
        apo_inf_sha256: String,
        apo_dll_sha256: String,
        apo_catalog_sha256: String,
        extension_inf_sha256: String,
        extension_catalog_sha256: String,
        extension_id: String,
        capx_context: String,
    }

    pub(crate) fn enrich_endpoints(
        endpoints: Vec<EndpointDescriptor>,
        helper: &Path,
        package_directory: &Path,
    ) -> Result<Vec<SystemAudioEndpoint>, String> {
        let topology_candidates = enumerate_topology_interfaces().unwrap_or_default();
        let mut runtime_by_id: HashMap<String, RuntimeResolution> = endpoints
            .iter()
            .map(|endpoint| {
                (
                    endpoint.id.to_ascii_lowercase(),
                    resolve_runtime(&endpoint.id, &topology_candidates),
                )
            })
            .collect();

        let input: Vec<_> = endpoints
            .iter()
            .map(|endpoint| {
                let runtime = runtime_by_id
                    .get(&endpoint.id.to_ascii_lowercase())
                    .expect("runtime resolution exists for every endpoint");
                InputEndpoint {
                    endpoint_id: &endpoint.id,
                    display_name: &endpoint.name,
                    is_default: endpoint.is_default,
                    runtime_device_id: runtime.binding_pnp_instance_id.as_deref(),
                    runtime_alias_match: runtime.alias_match,
                }
            })
            .collect();
        let resolved = run_fallback_helper(&input, helper)?;
        let mut by_id: HashMap<String, ResolvedEndpoint> = resolved
            .into_iter()
            .map(|item| (item.endpoint_id.to_ascii_lowercase(), item))
            .collect();

        Ok(endpoints
            .into_iter()
            .map(|endpoint| {
                let key = endpoint.id.to_ascii_lowercase();
                let runtime = runtime_by_id.remove(&key).unwrap_or(RuntimeResolution {
                    binding_pnp_instance_id: None,
                    topology_interface_path: None,
                    audio_interface_path: None,
                    kind: RuntimeBindingKind::None,
                    alias_match: false,
                });
                let mut item = by_id.remove(&key);

                if !fallback_device_matches_runtime(
                    runtime.binding_pnp_instance_id.as_deref(),
                    item.as_ref()
                        .and_then(|value| value.binding_pnp_instance_id.as_deref()),
                ) {
                    item = None;
                }

                let binding_pnp_instance_id = runtime
                    .binding_pnp_instance_id
                    .clone()
                    .or_else(|| {
                        item.as_ref()
                            .and_then(|value| value.binding_pnp_instance_id.clone())
                    });
                let adapter_name = item.as_ref().and_then(|value| value.adapter_name.clone());
                let pnp_instance_id = item
                    .as_ref()
                    .and_then(|value| value.pnp_instance_id.clone());
                let hardware_ids = item
                    .as_ref()
                    .map(|value| value.hardware_ids.clone())
                    .unwrap_or_default();
                let driver_inf = item.as_ref().and_then(|value| value.driver_inf.clone());
                let topology_references = item
                    .as_ref()
                    .map(|value| value.topology_references.clone())
                    .unwrap_or_default();
                let pnp_resolved = pnp_instance_id.is_some()
                    && !hardware_ids.is_empty()
                    && driver_inf.is_some();
                let topology_reference = (topology_references.len() == 1)
                    .then(|| topology_references[0].clone());
                let runtime_bound = matches!(runtime.kind, RuntimeBindingKind::Unique)
                    && runtime.topology_interface_path.is_some()
                    && runtime.audio_interface_path.is_some();
                let package_available = production_package_matches(
                    package_directory,
                    &hardware_ids,
                    runtime_bound,
                    topology_reference.as_deref(),
                );
                let status = classify_runtime_binding(
                    runtime.kind,
                    pnp_resolved,
                    &topology_references,
                    package_available,
                );
                let fallback_detail = item.and_then(|value| value.detail);
                let detail = resolution_detail(runtime.kind, status, fallback_detail);

                SystemAudioEndpoint {
                    endpoint_id: endpoint.id,
                    display_name: endpoint.name,
                    adapter_name,
                    is_default: endpoint.is_default,
                    binding_pnp_instance_id,
                    pnp_instance_id,
                    hardware_ids,
                    driver_inf,
                    topology_interface_path: runtime.topology_interface_path,
                    audio_interface_path: runtime.audio_interface_path,
                    topology_reference,
                    status,
                    detail,
                }
            })
            .collect())
    }

    fn resolve_runtime(
        endpoint_id: &str,
        candidates: &[TopologyCandidate],
    ) -> RuntimeResolution {
        let Ok(Some(adapter_device_id)) = resolve_adapter_device_id(endpoint_id) else {
            return empty_runtime(RuntimeBindingKind::None);
        };

        match select_topology_candidate(&adapter_device_id, candidates) {
            CandidateSelection::Unique(candidate) => {
                let alias_match = candidate.audio_interface_path.is_some();
                RuntimeResolution {
                    binding_pnp_instance_id: Some(candidate.device_instance_id),
                    topology_interface_path: Some(candidate.interface_path),
                    audio_interface_path: candidate.audio_interface_path,
                    kind: if alias_match {
                        RuntimeBindingKind::Unique
                    } else {
                        RuntimeBindingKind::None
                    },
                    alias_match,
                }
            }
            CandidateSelection::Ambiguous => empty_runtime(RuntimeBindingKind::Ambiguous),
            CandidateSelection::None => empty_runtime(RuntimeBindingKind::None),
        }
    }

    fn empty_runtime(kind: RuntimeBindingKind) -> RuntimeResolution {
        RuntimeResolution {
            binding_pnp_instance_id: None,
            topology_interface_path: None,
            audio_interface_path: None,
            kind,
            alias_match: false,
        }
    }

    fn run_fallback_helper(
        input: &[InputEndpoint<'_>],
        helper: &Path,
    ) -> Result<Vec<ResolvedEndpoint>, String> {
        if !helper.is_file() {
            return Ok(Vec::new());
        }
        let json = serde_json::to_vec(input)
            .map_err(|error| format!("failed to serialize Windows endpoints: {error}"))?;
        let mut command = Command::new("powershell.exe");
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(helper)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW);
        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start Windows endpoint discovery: {error}"))?;
        child
            .stdin
            .take()
            .ok_or_else(|| "Windows endpoint discovery stdin was unavailable".to_string())?
            .write_all(&json)
            .map_err(|error| format!("failed to send endpoints to discovery helper: {error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("failed to wait for Windows endpoint discovery: {error}"))?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if error.is_empty() {
                format!("Windows endpoint discovery exited with {}", output.status)
            } else {
                error
            });
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("Windows endpoint discovery returned invalid JSON: {error}"))
    }

    fn is_sha256(value: &str) -> bool {
        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    fn verified_apo_stage_matches(directory: &Path) -> bool {
        let path = directory.join("apo-verification.json");
        let Ok(bytes) = std::fs::read(path) else {
            return false;
        };
        let Ok(verification) = serde_json::from_slice::<ApoVerification>(&bytes) else {
            return false;
        };
        verification.extension_id.eq_ignore_ascii_case(EXPECTED_EXTENSION_ID)
            && verification.capx_context.eq_ignore_ascii_case(EXPECTED_CAPX_CONTEXT)
            && is_sha256(&verification.apo_inf_sha256)
            && is_sha256(&verification.apo_dll_sha256)
            && is_sha256(&verification.apo_catalog_sha256)
            && is_sha256(&verification.extension_inf_sha256)
            && is_sha256(&verification.extension_catalog_sha256)
    }

    fn production_package_matches(
        directory: &Path,
        hardware_ids: &[String],
        runtime_bound: bool,
        topology_reference: Option<&str>,
    ) -> bool {
        let extension = directory.join("VoxveilApoExtension.inf");
        for required in [
            "VoxveilApo.inf",
            "VoxveilApo.dll",
            "VoxveilApo.cat",
            "VoxveilApoExtension.inf",
            "VoxveilApoExtension.cat",
        ] {
            if !directory.join(required).is_file() {
                return false;
            }
        }
        if !verified_apo_stage_matches(directory) {
            return false;
        }
        if !runtime_bound && topology_reference.is_none() {
            return false;
        }
        std::fs::read_to_string(extension)
            .map(|text| capx_extension_inf_matches(&text, hardware_ids, topology_reference))
            .unwrap_or(false)
    }

    fn resolution_detail(
        runtime_kind: RuntimeBindingKind,
        status: SystemAudioEndpointStatus,
        fallback_detail: Option<String>,
    ) -> Option<String> {
        match runtime_kind {
            RuntimeBindingKind::Ambiguous => default_detail(SystemAudioEndpointStatus::Ambiguous),
            RuntimeBindingKind::Unique => default_detail(status),
            RuntimeBindingKind::None => fallback_detail.or_else(|| default_detail(status)),
        }
    }

    fn default_detail(status: SystemAudioEndpointStatus) -> Option<String> {
        match status {
            SystemAudioEndpointStatus::ComponentRequired => Some(
                "Output identified, but this build has no verified matching signed CAPX extension package for its driver"
                    .into(),
            ),
            SystemAudioEndpointStatus::Ambiguous => Some(
                "More than one topology binding matched this output; Voxveil will not guess".into(),
            ),
            SystemAudioEndpointStatus::Unsupported => {
                Some("Voxveil could not safely resolve this driver's topology binding".into())
            }
            _ => None,
        }
    }
}

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
        assert_eq!(
            classify_binding(true, &["Topology".into()], true),
            SystemAudioEndpointStatus::Installable
        );
    }

    #[test]
    fn multiple_topology_candidates_fail_closed() {
        assert_eq!(
            classify_binding(
                true,
                &["Topology".into(), "HeadphoneTopology".into()],
                true
            ),
            SystemAudioEndpointStatus::Ambiguous
        );
    }

    #[test]
    fn missing_pnp_identity_is_unsupported() {
        assert_eq!(
            classify_binding(false, &["Topology".into()], true),
            SystemAudioEndpointStatus::Unsupported
        );
    }

    #[test]
    fn resolved_binding_without_installable_package_requires_component() {
        assert_eq!(
            classify_binding(true, &["Topology".into()], false),
            SystemAudioEndpointStatus::ComponentRequired
        );
    }

    #[test]
    fn signed_extension_must_match_hardware_and_topology_for_fallback() {
        let hardware_ids: Vec<String> = vec!["HDAUDIO\\FUNC_01&VEN_10EC".into()];
        let text = capx_inf(&hardware_ids[0], "PrimaryLineOutTopo");
        assert!(capx_extension_inf_matches(
            &text,
            &hardware_ids,
            Some("PrimaryLineOutTopo")
        ));
        assert!(!capx_extension_inf_matches(
            &text,
            &hardware_ids,
            Some("HeadphoneTopo")
        ));
        assert!(!capx_extension_inf_matches(
            &text,
            &["USB\\VID_1234".into()],
            Some("PrimaryLineOutTopo")
        ));
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
