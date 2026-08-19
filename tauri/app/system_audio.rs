use std::path::{Path, PathBuf};

use tauri::State;

use super::dto::{InstallResultDto, SystemAudioEndpointDto};
use crate::platform::ProcessingController;

#[cfg(target_os = "windows")]
use serde::Serialize;
#[cfg(target_os = "windows")]
use voxveil_windows_audio::{SystemAudioEndpoint, SystemAudioEndpointStatus};

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
    if endpoint.pnp_instance_id.is_none()
        || endpoint.hardware_ids.is_empty()
        || endpoint.driver_inf.is_none()
        || endpoint.topology_reference.is_none()
    {
        return Err("The selected playback endpoint no longer has a complete driver binding.".into());
    }
    Ok(endpoint)
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
    pnp_instance_id: String,
    hardware_id: String,
    hardware_ids: Vec<String>,
    driver_inf: String,
    topology_reference: String,
}

#[cfg(target_os = "windows")]
impl TryFrom<SystemAudioEndpoint> for EndpointInstallDescriptor {
    type Error = String;

    fn try_from(endpoint: SystemAudioEndpoint) -> Result<Self, Self::Error> {
        Ok(Self {
            endpoint_id: endpoint.endpoint_id,
            pnp_instance_id: endpoint
                .pnp_instance_id
                .ok_or_else(|| "missing PnP instance ID".to_string())?,
            hardware_id: endpoint
                .hardware_ids
                .first()
                .cloned()
                .ok_or_else(|| "missing hardware ID".to_string())?,
            hardware_ids: endpoint.hardware_ids,
            driver_inf: endpoint
                .driver_inf
                .ok_or_else(|| "missing installed driver INF".to_string())?,
            topology_reference: endpoint
                .topology_reference
                .ok_or_else(|| "missing topology reference".to_string())?,
        })
    }
}

#[cfg(target_os = "windows")]
fn system_audio_installer_path(executable: &Path) -> Result<PathBuf, String> {
    let directory = executable
        .parent()
        .ok_or_else(|| "Voxveil executable has no parent directory".to_string())?;
    Ok(directory
        .join("system-audio")
        .join("install-system-audio-component.ps1"))
}

#[cfg(target_os = "windows")]
fn temporary_descriptor_path() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("voxveil-endpoint-{}-{nonce}.json", std::process::id()))
}

#[cfg(target_os = "windows")]
fn powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(target_os = "windows")]
fn launch_system_audio_installer(script: &Path, descriptor: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script = powershell_single_quoted(&script.to_string_lossy());
    let descriptor = powershell_single_quoted(&descriptor.to_string_lossy());
    let launch = format!(
        r#"$ErrorActionPreference='Stop'; $script='{script}'; $descriptor='{descriptor}'; $scriptArg='"' + $script + '"'; $descriptorArg='"' + $descriptor + '"'; try {{ $process=Start-Process -FilePath 'powershell.exe' -Verb RunAs -Wait -PassThru -ArgumentList @('-NoLogo','-NoProfile','-ExecutionPolicy','Bypass','-File',$scriptArg,'-EndpointDescriptor',$descriptorArg); exit $process.ExitCode }} catch {{ Write-Error $_; exit 1 }}"#,
    );
    let status = std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &launch,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|error| format!("failed to open the system-audio installer: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("The system-audio installer was cancelled or exited with an error.".into())
    }
}

#[tauri::command]
pub fn install_system_audio_component(
    controller: State<'_, ProcessingController>,
    endpoint_id: String,
) -> Result<InstallResultDto, String> {
    #[cfg(target_os = "windows")]
    {
        let endpoint = select_installable_endpoint(controller.system_audio_endpoints()?, &endpoint_id)?;
        let descriptor = EndpointInstallDescriptor::try_from(endpoint)?;
        let executable = std::env::current_exe()
            .map_err(|error| format!("failed to locate the Voxveil executable: {error}"))?;
        let script = system_audio_installer_path(&executable)?;
        if !script.is_file() {
            return Err(format!("Bundled system-audio installer not found at {}.", script.display()));
        }
        let descriptor_path = temporary_descriptor_path();
        let json = serde_json::to_vec_pretty(&descriptor)
            .map_err(|error| format!("failed to serialize endpoint descriptor: {error}"))?;
        std::fs::write(&descriptor_path, json)
            .map_err(|error| format!("failed to create endpoint descriptor: {error}"))?;
        let result = launch_system_audio_installer(&script, &descriptor_path);
        let _ = std::fs::remove_file(&descriptor_path);
        result?;
        return Ok(InstallResultDto {
            endpoint_id,
            outcome: "launched".into(),
            detail: None,
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (controller, endpoint_id);
        Err("The system-audio component installer is available only on Windows.".into())
    }
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

    #[test]
    fn resolves_installer_beside_packaged_executable() {
        let executable = PathBuf::from("bundle").join("voxveil.exe");
        assert_eq!(
            system_audio_installer_path(&executable),
            Ok(PathBuf::from("bundle")
                .join("system-audio")
                .join("install-system-audio-component.ps1"))
        );
    }

    #[test]
    fn escapes_apostrophes_for_powershell_single_quoted_strings() {
        assert_eq!(
            powershell_single_quoted("C:\\User's Files\\setup.ps1"),
            "C:\\User''s Files\\setup.ps1"
        );
    }
}
