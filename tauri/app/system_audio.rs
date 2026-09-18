use std::path::{Path, PathBuf};

use tauri::State;

use super::dto::{InstallResultDto, SystemAudioEndpointDto};
use crate::platform::ProcessingController;

#[cfg(target_os = "windows")]
#[path = "system_audio_installer.rs"]
mod system_audio_installer;
#[cfg(target_os = "windows")]
use system_audio_installer::{launch_system_audio_installer, InstallerLaunchOutcome};

#[cfg(target_os = "windows")]
use serde::Serialize;
#[cfg(target_os = "windows")]
use sha2::{Digest, Sha256};
#[cfg(target_os = "windows")]
use voxveil_windows_audio::{SystemAudioEndpoint, SystemAudioEndpointStatus};

#[cfg(target_os = "windows")]
fn has_runtime_interface_binding(endpoint: &SystemAudioEndpoint) -> bool {
    endpoint.topology_interface_path.is_some() && endpoint.audio_interface_path.is_some()
}

#[cfg(target_os = "windows")]
fn select_installable_endpoint(
    endpoints: Vec<SystemAudioEndpoint>,
    endpoint_id: &str,
) -> Result<SystemAudioEndpoint, String> {
    let endpoint = endpoints
        .into_iter()
        .find(|endpoint| endpoint.endpoint_id == endpoint_id)
        .ok_or_else(|| "The selected playback endpoint is no longer available. Refresh and try again.".to_string())?;
    if endpoint.status != SystemAudioEndpointStatus::Installable {
        return Err("The selected playback endpoint is not installable by this Voxveil package.".into());
    }
    let binding_complete = endpoint.binding_pnp_instance_id.is_some()
        && endpoint.pnp_instance_id.is_some()
        && !endpoint.hardware_ids.is_empty()
        && endpoint.driver_inf.is_some()
        && (has_runtime_interface_binding(&endpoint) || endpoint.topology_reference.is_some());
    if !binding_complete {
        return Err("The selected playback endpoint no longer has a complete driver binding.".into());
    }
    Ok(endpoint)
}

#[cfg(target_os = "windows")]
pub(super) fn validate_revalidated_binding(
    selected: &SystemAudioEndpoint,
    current: &SystemAudioEndpoint,
) -> Result<(), String> {
    let changed = current.status != SystemAudioEndpointStatus::Installable
        || selected.endpoint_id != current.endpoint_id
        || !same_optional_value(
            selected.binding_pnp_instance_id.as_deref(),
            current.binding_pnp_instance_id.as_deref(),
        )
        || !same_optional_value(
            selected.pnp_instance_id.as_deref(),
            current.pnp_instance_id.as_deref(),
        )
        || !same_optional_value(selected.driver_inf.as_deref(), current.driver_inf.as_deref())
        || !same_optional_value(
            selected.topology_interface_path.as_deref(),
            current.topology_interface_path.as_deref(),
        )
        || !same_optional_value(
            selected.audio_interface_path.as_deref(),
            current.audio_interface_path.as_deref(),
        )
        || !same_optional_value(
            selected.topology_reference.as_deref(),
            current.topology_reference.as_deref(),
        )
        || normalized_hardware_ids(&selected.hardware_ids)
            != normalized_hardware_ids(&current.hardware_ids);
    if changed {
        Err("The playback endpoint binding changed during installation setup. Refresh and try again.".into())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn same_optional_value(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        (None, None) => true,
        _ => false,
    }
}

#[cfg(target_os = "windows")]
fn normalized_hardware_ids(values: &[String]) -> Vec<String> {
    let mut values: Vec<_> = values.iter().map(|value| value.to_ascii_lowercase()).collect();
    values.sort_unstable();
    values.dedup();
    values
}

#[cfg(target_os = "windows")]
fn endpoint_dto(endpoint: SystemAudioEndpoint) -> SystemAudioEndpointDto {
    SystemAudioEndpointDto {
        endpoint_id: endpoint.endpoint_id,
        display_name: endpoint.display_name,
        adapter_name: endpoint.adapter_name,
        is_default: endpoint.is_default,
        status: status_name(endpoint.status).into(),
        detail: endpoint.detail,
    }
}

#[cfg(target_os = "windows")]
fn status_name(status: SystemAudioEndpointStatus) -> &'static str {
    match status {
        SystemAudioEndpointStatus::Ready => "ready",
        SystemAudioEndpointStatus::Installable => "installable",
        SystemAudioEndpointStatus::ComponentRequired => "component-required",
        SystemAudioEndpointStatus::Ambiguous => "ambiguous",
        SystemAudioEndpointStatus::Unsupported => "unsupported",
    }
}

#[tauri::command]
pub fn list_system_audio_endpoints(
    controller: State<'_, ProcessingController>,
) -> Result<Vec<SystemAudioEndpointDto>, String> {
    #[cfg(target_os = "windows")]
    {
        return controller
            .system_audio_endpoints()
            .map(|endpoints| endpoints.into_iter().map(endpoint_dto).collect());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = controller;
        Ok(Vec::new())
    }
}

#[cfg(target_os = "windows")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EndpointInstallDescriptor {
    endpoint_id: String,
    binding_pnp_instance_id: String,
    pnp_instance_id: String,
    hardware_id: String,
    hardware_ids: Vec<String>,
    driver_inf: String,
    topology_interface_path: Option<String>,
    audio_interface_path: Option<String>,
    topology_reference: Option<String>,
}

#[cfg(target_os = "windows")]
impl TryFrom<SystemAudioEndpoint> for EndpointInstallDescriptor {
    type Error = String;

    fn try_from(endpoint: SystemAudioEndpoint) -> Result<Self, Self::Error> {
        let topology_interface_path = endpoint.topology_interface_path;
        let audio_interface_path = endpoint.audio_interface_path;
        let topology_reference = endpoint.topology_reference;
        let has_runtime_binding = topology_interface_path.is_some() && audio_interface_path.is_some();
        if topology_interface_path.is_some() != audio_interface_path.is_some() {
            return Err("incomplete runtime device-interface binding".into());
        }
        if !has_runtime_binding && topology_reference.is_none() {
            return Err("missing runtime interface binding and topology reference fallback".into());
        }

        Ok(Self {
            endpoint_id: endpoint.endpoint_id,
            binding_pnp_instance_id: endpoint
                .binding_pnp_instance_id
                .ok_or_else(|| "missing topology binding PnP instance ID".to_string())?,
            pnp_instance_id: endpoint
                .pnp_instance_id
                .ok_or_else(|| "missing driver metadata PnP instance ID".to_string())?,
            hardware_id: endpoint
                .hardware_ids
                .first()
                .cloned()
                .ok_or_else(|| "missing hardware ID".to_string())?,
            hardware_ids: endpoint.hardware_ids,
            driver_inf: endpoint
                .driver_inf
                .ok_or_else(|| "missing installed driver INF".to_string())?,
            topology_interface_path,
            audio_interface_path,
            topology_reference,
        })
    }
}

#[cfg(target_os = "windows")]
fn system_audio_installer_path(executable: &Path) -> Result<PathBuf, String> {
    let directory = executable
        .parent()
        .ok_or_else(|| "Voxveil executable has no parent directory".to_string())?;
    Ok(directory.join("system-audio").join("install-system-audio-component.ps1"))
}

#[cfg(target_os = "windows")]
fn create_temporary_descriptor(json: &[u8]) -> Result<PathBuf, String> {
    use std::fs::OpenOptions;
    use std::io::{ErrorKind, Write};
    use std::time::{SystemTime, UNIX_EPOCH};

    for attempt in 0..16u8 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "voxveil-endpoint-{}-{nonce}-{attempt}.json",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(json) {
                    let _ = std::fs::remove_file(&path);
                    return Err(format!("failed to write endpoint descriptor: {error}"));
                }
                return Ok(path);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!("failed to create endpoint descriptor: {error}"));
            }
        }
    }

    Err("failed to create a unique endpoint descriptor after repeated path collisions".into())
}

#[cfg(target_os = "windows")]
fn powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

#[tauri::command]
pub fn install_system_audio_component(
    controller: State<'_, ProcessingController>,
    endpoint_id: String,
) -> Result<InstallResultDto, String> {
    #[cfg(target_os = "windows")]
    {
        let selected = select_installable_endpoint(controller.system_audio_endpoints()?, &endpoint_id)?;
        let current = select_installable_endpoint(controller.system_audio_endpoints()?, &endpoint_id)
            .map_err(|_| {
                "The playback endpoint binding changed during installation setup. Refresh and try again."
                    .to_string()
            })?;
        validate_revalidated_binding(&selected, &current)?;
        let descriptor = EndpointInstallDescriptor::try_from(current)?;
        let executable = std::env::current_exe()
            .map_err(|error| format!("failed to locate the Voxveil executable: {error}"))?;
        let script = system_audio_installer_path(&executable)?;
        if !script.is_file() {
            return Err(format!("Bundled system-audio installer not found at {}.", script.display()));
        }
        let json = serde_json::to_vec_pretty(&descriptor)
            .map_err(|error| format!("failed to serialize endpoint descriptor: {error}"))?;
        let descriptor_sha256 = format!("{:x}", Sha256::digest(&json));
        let descriptor_path = create_temporary_descriptor(&json)?;
        let result =
            launch_system_audio_installer(&script, &descriptor_path, &descriptor_sha256);
        let _ = std::fs::remove_file(&descriptor_path);
        let outcome = result?;
        return Ok(match outcome {
            InstallerLaunchOutcome::Completed => InstallResultDto {
                endpoint_id,
                outcome: "launched".into(),
                detail: None,
            },
            InstallerLaunchOutcome::RebootRequired => InstallResultDto {
                endpoint_id,
                outcome: "reboot-required".into(),
                detail: Some(
                    "Windows must restart to finish installing the Voxveil system-audio component. Restart Windows, then run this installation again."
                        .into(),
                ),
            },
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (controller, endpoint_id);
        Err("The system-audio component installer is available only on Windows.".into())
    }
}

#[cfg(all(test, target_os = "windows"))]
#[path = "system_audio_tests.rs"]
mod tests;
