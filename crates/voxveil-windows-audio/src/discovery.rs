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

#[cfg(windows)]
mod windows {
    use std::collections::HashMap;
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::{Command, Stdio};

    use serde::{Deserialize, Serialize};

    use super::{SystemAudioEndpoint, SystemAudioEndpointStatus, classify_binding};
    use crate::device::EndpointDescriptor;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct InputEndpoint<'a> {
        endpoint_id: &'a str,
        display_name: &'a str,
        is_default: bool,
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
        package_available: bool,
    ) -> Result<Vec<SystemAudioEndpoint>, String> {
        if !helper.is_file() {
            return Ok(endpoints
                .into_iter()
                .map(|endpoint| unsupported(endpoint, "Windows endpoint discovery helper is missing"))
                .collect());
        }

        let input: Vec<_> = endpoints
            .iter()
            .map(|endpoint| InputEndpoint {
                endpoint_id: &endpoint.id,
                display_name: &endpoint.name,
                is_default: endpoint.is_default,
            })
            .collect();
        let json = serde_json::to_vec(&input)
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

        let resolved: Vec<ResolvedEndpoint> = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("Windows endpoint discovery returned invalid JSON: {error}"))?;
        let mut by_id: HashMap<String, ResolvedEndpoint> = resolved
            .into_iter()
            .map(|item| (item.endpoint_id.to_ascii_lowercase(), item))
            .collect();

        Ok(endpoints
            .into_iter()
            .map(|endpoint| {
                let Some(item) = by_id.remove(&endpoint.id.to_ascii_lowercase()) else {
                    return unsupported(endpoint, "Windows could not resolve this playback endpoint");
                };
                let pnp_resolved = item.pnp_instance_id.is_some()
                    && !item.hardware_ids.is_empty()
                    && item.driver_inf.is_some();
                let status = classify_binding(
                    pnp_resolved,
                    &item.topology_references,
                    package_available,
                );
                let topology_reference = (item.topology_references.len() == 1)
                    .then(|| item.topology_references[0].clone());
                let detail = item.detail.or_else(|| default_detail(status));
                SystemAudioEndpoint {
                    endpoint_id: endpoint.id,
                    display_name: endpoint.name,
                    adapter_name: item.adapter_name,
                    is_default: endpoint.is_default,
                    pnp_instance_id: item.pnp_instance_id,
                    hardware_ids: item.hardware_ids,
                    driver_inf: item.driver_inf,
                    topology_reference,
                    status,
                    detail,
                }
            })
            .collect())
    }

    fn unsupported(endpoint: EndpointDescriptor, detail: &str) -> SystemAudioEndpoint {
        SystemAudioEndpoint {
            endpoint_id: endpoint.id,
            display_name: endpoint.name,
            adapter_name: None,
            is_default: endpoint.is_default,
            pnp_instance_id: None,
            hardware_ids: Vec::new(),
            driver_inf: None,
            topology_reference: None,
            status: SystemAudioEndpointStatus::Unsupported,
            detail: Some(detail.into()),
        }
    }

    fn default_detail(status: SystemAudioEndpointStatus) -> Option<String> {
        match status {
            SystemAudioEndpointStatus::ComponentRequired => Some(
                "Output identified, but this build has no signed extension package for its driver"
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
}
