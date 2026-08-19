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
    pub pnp_instance_id: Option<String>,
    pub hardware_ids: Vec<String>,
    pub driver_inf: Option<String>,
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

pub(crate) fn extension_inf_matches(
    text: &str,
    hardware_ids: &[String],
    topology_reference: &str,
) -> bool {
    if hardware_ids.is_empty() || topology_reference.is_empty() {
        return false;
    }
    let text = text.to_ascii_lowercase();
    let topology = topology_reference.to_ascii_lowercase();
    hardware_ids
        .iter()
        .any(|hardware_id| text.contains(&hardware_id.to_ascii_lowercase()))
        && text.contains(&topology)
}

#[cfg(windows)]
mod windows {
    use std::collections::HashMap;
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::{Command, Stdio};

    use serde::{Deserialize, Serialize};

    use super::{SystemAudioEndpoint, SystemAudioEndpointStatus, extension_inf_matches};
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

    #[derive(Clone, Debug)]
    struct RuntimeResolution {
        device_id: Option<String>,
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
        adapter_name: Option<String>,
        pnp_instance_id: Option<String>,
        #[serde(default)]
        hardware_ids: Vec<String>,
        driver_inf: Option<String>,
        #[serde(default)]
        topology_references: Vec<String>,
        detail: Option<String>,
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
                    runtime_device_id: runtime.device_id.as_deref(),
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
                    device_id: None,
                    kind: RuntimeBindingKind::None,
                    alias_match: false,
                });
                let mut item = by_id.remove(&key);

                if !fallback_device_matches_runtime(
                    runtime.device_id.as_deref(),
                    item.as_ref().and_then(|value| value.pnp_instance_id.as_deref()),
                ) {
                    item = None;
                }

                let adapter_name = item.as_ref().and_then(|value| value.adapter_name.clone());
                let pnp_instance_id = item
                    .as_ref()
                    .and_then(|value| value.pnp_instance_id.clone())
                    .or_else(|| runtime.device_id.clone());
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
                let package_available = topology_reference.as_deref().is_some_and(|reference| {
                    production_package_matches(package_directory, &hardware_ids, reference)
                });
                let status = classify_runtime_binding(
                    runtime.kind,
                    pnp_resolved,
                    &topology_references,
                    package_available,
                );
                let fallback_detail = item.and_then(|value| value.detail);
                let detail = resolution_detail(
                    runtime.kind,
                    status,
                    &topology_references,
                    fallback_detail,
                );

                SystemAudioEndpoint {
                    endpoint_id: endpoint.id,
                    display_name: endpoint.name,
                    adapter_name,
                    is_default: endpoint.is_default,
                    pnp_instance_id,
                    hardware_ids,
                    driver_inf,
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
        let Ok(Some(device_id)) = resolve_adapter_device_id(endpoint_id) else {
            return RuntimeResolution {
                device_id: None,
                kind: RuntimeBindingKind::None,
                alias_match: false,
            };
        };

        match select_topology_candidate(&device_id, candidates) {
            CandidateSelection::Unique(candidate) => {
                let _runtime_interface_path = &candidate.interface_path;
                RuntimeResolution {
                    device_id: Some(device_id),
                    kind: RuntimeBindingKind::Unique,
                    alias_match: candidate.alias_match,
                }
            }
            CandidateSelection::Ambiguous => RuntimeResolution {
                device_id: Some(device_id),
                kind: RuntimeBindingKind::Ambiguous,
                alias_match: false,
            },
            CandidateSelection::None => RuntimeResolution {
                device_id: Some(device_id),
                kind: RuntimeBindingKind::None,
                alias_match: false,
            },
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

    fn production_package_matches(
        directory: &Path,
        hardware_ids: &[String],
        topology_reference: &str,
    ) -> bool {
        let extension = directory.join("VoxveilApoExtension.inf");
        if !directory.join("VoxveilApo.cat").is_file()
            || !directory.join("VoxveilApoExtension.cat").is_file()
            || !extension.is_file()
        {
            return false;
        }
        std::fs::read_to_string(extension)
            .map(|text| extension_inf_matches(&text, hardware_ids, topology_reference))
            .unwrap_or(false)
    }

    fn resolution_detail(
        runtime_kind: RuntimeBindingKind,
        status: SystemAudioEndpointStatus,
        topology_references: &[String],
        fallback_detail: Option<String>,
    ) -> Option<String> {
        match runtime_kind {
            RuntimeBindingKind::Ambiguous => default_detail(SystemAudioEndpointStatus::Ambiguous),
            RuntimeBindingKind::Unique if topology_references.is_empty() => Some(
                "Windows resolved this output topology at runtime, but the installed driver did not expose the literal reference string required by the extension package."
                    .into(),
            ),
            RuntimeBindingKind::Unique => default_detail(status),
            RuntimeBindingKind::None => fallback_detail.or_else(|| default_detail(status)),
        }
    }

    fn default_detail(status: SystemAudioEndpointStatus) -> Option<String> {
        match status {
            SystemAudioEndpointStatus::ComponentRequired => Some(
                "Output identified, but this build has no matching signed extension package for its driver"
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
    fn signed_extension_must_match_hardware_and_topology() {
        let text = "HardwareId=HDAUDIO\\FUNC_01&VEN_10EC\nReference=PrimaryLineOutTopo";
        let hardware_ids = vec!["HDAUDIO\\FUNC_01&VEN_10EC".into()];
        assert!(extension_inf_matches(text, &hardware_ids, "PrimaryLineOutTopo"));
        assert!(!extension_inf_matches(text, &hardware_ids, "HeadphoneTopo"));
        assert!(!extension_inf_matches(
            text,
            &["USB\\VID_1234".into()],
            "PrimaryLineOutTopo"
        ));
    }
}
