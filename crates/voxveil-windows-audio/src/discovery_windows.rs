use std::collections::HashMap;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use super::{SystemAudioEndpoint, SystemAudioEndpointStatus, capx_extension_inf_matches};
use crate::binding::{RuntimeBindingKind, classify_runtime_binding, fallback_device_matches_runtime};
use crate::device::EndpointDescriptor;
use crate::device_interfaces::{CandidateSelection, TopologyCandidate, enumerate_topology_interfaces, select_topology_candidate};
use crate::topology::{resolve_adapter_device_id, windows_system_directory};

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
    apo_signer: String,
    apo_thumbprint: String,
    apo_catalog_signer: String,
    apo_catalog_thumbprint: String,
    extension_catalog_signer: String,
    extension_catalog_thumbprint: String,
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
            let runtime = runtime_by_id
                .remove(&key)
                .unwrap_or_else(|| empty_runtime(RuntimeBindingKind::None));
            let mut item = by_id.remove(&key);
            if !fallback_device_matches_runtime(
                runtime.binding_pnp_instance_id.as_deref(),
                item.as_ref().and_then(|value| value.binding_pnp_instance_id.as_deref()),
            ) {
                item = None;
            }

            let binding_pnp_instance_id = runtime.binding_pnp_instance_id.clone().or_else(|| {
                item.as_ref().and_then(|value| value.binding_pnp_instance_id.clone())
            });
            let adapter_name = item.as_ref().and_then(|value| value.adapter_name.clone());
            let pnp_instance_id = item.as_ref().and_then(|value| value.pnp_instance_id.clone());
            let hardware_ids = item.as_ref().map(|value| value.hardware_ids.clone()).unwrap_or_default();
            let driver_inf = item.as_ref().and_then(|value| value.driver_inf.clone());
            let topology_references = item
                .as_ref()
                .map(|value| value.topology_references.clone())
                .unwrap_or_default();
            let pnp_resolved =
                pnp_instance_id.is_some() && !hardware_ids.is_empty() && driver_inf.is_some();
            let topology_reference =
                (topology_references.len() == 1).then(|| topology_references[0].clone());
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
            let detail = resolution_detail(runtime.kind, status, item.and_then(|value| value.detail));

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

fn resolve_runtime(endpoint_id: &str, candidates: &[TopologyCandidate]) -> RuntimeResolution {
    let Ok(Some(adapter_device_id)) = resolve_adapter_device_id(endpoint_id) else { return empty_runtime(RuntimeBindingKind::None); };
    match select_topology_candidate(&adapter_device_id, candidates) {
        CandidateSelection::Unique(candidate) => {
            let alias_match = candidate.audio_interface_path.is_some();
            RuntimeResolution {
                binding_pnp_instance_id: Some(candidate.device_instance_id),
                topology_interface_path: Some(candidate.interface_path),
                audio_interface_path: candidate.audio_interface_path,
                kind: if alias_match { RuntimeBindingKind::Unique } else { RuntimeBindingKind::None },
                alias_match,
            }
        }
        CandidateSelection::Ambiguous => empty_runtime(RuntimeBindingKind::Ambiguous), CandidateSelection::None => empty_runtime(RuntimeBindingKind::None),
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
    let powershell = windows_system_directory()?.join(r"WindowsPowerShell\v1.0\powershell.exe");
    if !powershell.is_file() {
        return Err(format!("Windows PowerShell was not found at {}.", powershell.display()));
    }
    let mut command = Command::new(&powershell);
    command
        .args(["-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
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

fn is_certificate_thumbprint(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn verified_apo_stage_matches(directory: &Path) -> bool {
    let path = directory.join("apo-verification.json");
    let Ok(bytes) = std::fs::read(path) else { return false; };
    let Ok(verification) = serde_json::from_slice::<ApoVerification>(&bytes) else { return false; };
    verification.extension_id.eq_ignore_ascii_case(EXPECTED_EXTENSION_ID)
        && verification.capx_context.eq_ignore_ascii_case(EXPECTED_CAPX_CONTEXT)
        && is_sha256(&verification.apo_inf_sha256)
        && is_sha256(&verification.apo_dll_sha256)
        && is_sha256(&verification.apo_catalog_sha256)
        && is_sha256(&verification.extension_inf_sha256)
        && is_sha256(&verification.extension_catalog_sha256)
        && !verification.apo_signer.trim().is_empty()
        && is_certificate_thumbprint(&verification.apo_thumbprint)
        && !verification.apo_catalog_signer.trim().is_empty()
        && is_certificate_thumbprint(&verification.apo_catalog_thumbprint)
        && !verification.extension_catalog_signer.trim().is_empty()
        && is_certificate_thumbprint(&verification.extension_catalog_thumbprint)
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
        if !directory.join(required).is_file() { return false; }
    }
    if !verified_apo_stage_matches(directory) || (!runtime_bound && topology_reference.is_none()) { return false; }
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
            "Output identified, but this build has no verified matching signed CAPX extension package for its driver".into(),
        ),
        SystemAudioEndpointStatus::Ambiguous => {
            Some("More than one topology binding matched this output; Voxveil will not guess".into())
        }
        SystemAudioEndpointStatus::Unsupported => {
            Some("Voxveil could not safely resolve this driver's topology binding".into())
        }
        _ => None,
    }
}
