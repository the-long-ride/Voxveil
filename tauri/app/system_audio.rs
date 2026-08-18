use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
use std::{fs, path::Path, process::Command};

const APO_DLL: &[u8] = include_bytes!("../generated-system-audio/VoxveilApo.dll");
const APO_CHECKER: &[u8] = include_bytes!("../generated-system-audio/VoxveilApoCheck.exe");
const INSTALL_SCRIPT: &str = include_str!("../../native/windows/apo/install.ps1");
const UNINSTALL_SCRIPT: &str = include_str!("../../native/windows/apo/uninstall.ps1");

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAudioInstallError {
    message: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Deserialize)]
struct InstallerResult {
    success: bool,
    message: String,
    details: String,
}

fn decode_process_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

fn installer_error_from_output(
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
    elevated_details: Option<&str>,
) -> SystemAudioInstallError {
    let mut stderr = decode_process_text(stderr);
    if let Some(details) = elevated_details.map(str::trim).filter(|value| !value.is_empty()) {
        if !stderr.is_empty() {
            stderr.push_str("\n\nElevated installer details:\n");
        }
        stderr.push_str(details);
    }
    SystemAudioInstallError {
        message: "Windows system-audio installation failed".to_string(),
        exit_code,
        stdout: decode_process_text(stdout),
        stderr,
    }
}

fn simple_install_error(message: impl Into<String>) -> SystemAudioInstallError {
    SystemAudioInstallError {
        message: message.into(),
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
    }
}

fn validate_pe_payload(name: &str, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 2 || &bytes[..2] != b"MZ" {
        return Err(format!(
            "embedded Windows system-audio payload is invalid: {name}"
        ));
    }
    Ok(())
}

pub fn verify_embedded_payload() -> Result<(), String> {
    validate_pe_payload("VoxveilApo.dll", APO_DLL)?;
    validate_pe_payload("VoxveilApoCheck.exe", APO_CHECKER)?;
    if INSTALL_SCRIPT.trim().is_empty() || UNINSTALL_SCRIPT.trim().is_empty() {
        return Err("embedded Windows system-audio scripts are empty".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn stage_embedded_package(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("failed to create temporary system-audio package: {error}"))?;
    for (name, bytes) in [
        ("VoxveilApo.dll", APO_DLL),
        ("VoxveilApoCheck.exe", APO_CHECKER),
    ] {
        fs::write(root.join(name), bytes)
            .map_err(|error| format!("failed to stage embedded {name}: {error}"))?;
    }
    for (name, text) in [
        ("install.ps1", INSTALL_SCRIPT),
        ("uninstall.ps1", UNINSTALL_SCRIPT),
    ] {
        fs::write(root.join(name), text.as_bytes())
            .map_err(|error| format!("failed to stage embedded {name}: {error}"))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn temporary_package_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("voxveil-system-audio-{}", std::process::id()))
}

#[cfg(target_os = "windows")]
fn windows_powershell() -> std::path::PathBuf {
    std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
}

#[cfg(target_os = "windows")]
fn read_installer_result(path: &Path) -> Option<InstallerResult> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}').trim()).ok()
}

#[cfg(target_os = "windows")]
fn run_embedded_installer() -> Result<(), SystemAudioInstallError> {
    verify_embedded_payload().map_err(simple_install_error)?;
    let package = temporary_package_root();
    if package.exists() {
        fs::remove_dir_all(&package).map_err(|error| {
            simple_install_error(format!(
                "failed to reset temporary system-audio package: {error}"
            ))
        })?;
    }
    stage_embedded_package(&package).map_err(simple_install_error)?;

    let installer = package.join("install.ps1");
    let result_path = package.join("install-result.json");
    let output = Command::new(windows_powershell())
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&installer)
        .arg("-PackageRoot")
        .arg(&package)
        .arg("-ResultPath")
        .arg(&result_path)
        .output()
        .map_err(|error| {
            simple_install_error(format!(
                "failed to launch embedded Windows system-audio installer: {error}"
            ))
        })?;

    let elevated = read_installer_result(&result_path);
    let elevated_failed = elevated.as_ref().is_some_and(|result| !result.success);
    if !output.status.success() || elevated_failed {
        let details = elevated.as_ref().map(|result| {
            if result.details.trim().is_empty() {
                result.message.as_str()
            } else {
                result.details.as_str()
            }
        });
        let error = installer_error_from_output(
            output.status.code(),
            &output.stdout,
            &output.stderr,
            details,
        );
        let _ = fs::remove_dir_all(&package);
        return Err(error);
    }

    fs::remove_dir_all(&package).map_err(|error| {
        simple_install_error(format!(
            "system-audio component installed, but temporary package cleanup failed: {error}"
        ))
    })?;
    Ok(())
}

#[tauri::command]
pub fn install_windows_audio_component() -> Result<(), SystemAudioInstallError> {
    #[cfg(target_os = "windows")]
    {
        run_embedded_installer()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(simple_install_error(
            "Windows system-audio installation is unavailable on this platform",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_embedded_binary_is_rejected() {
        assert!(validate_pe_payload("broken.dll", b"not-a-pe").is_err());
    }

    #[test]
    fn valid_pe_signature_is_accepted() {
        assert!(validate_pe_payload("payload.dll", b"MZpayload").is_ok());
    }

    #[test]
    fn installer_failure_preserves_process_and_elevated_details() {
        let error = installer_error_from_output(
            Some(1),
            b"Preparing Voxveil APO\n",
            b"PowerShell failed\n",
            Some("Access to the registry key is denied."),
        );
        assert_eq!(error.exit_code, Some(1));
        assert_eq!(error.stdout, "Preparing Voxveil APO");
        assert!(error.stderr.contains("PowerShell failed"));
        assert!(error.stderr.contains("Access to the registry key is denied."));
    }
}
