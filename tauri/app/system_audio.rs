#[cfg(target_os = "windows")]
use std::{fs, path::Path, process::Command};

const APO_DLL: &[u8] = include_bytes!("../generated-system-audio/VoxveilApo.dll");
const APO_CHECKER: &[u8] = include_bytes!("../generated-system-audio/VoxveilApoCheck.exe");
const INSTALL_SCRIPT: &str = include_str!("../../native/windows/apo/install.ps1");
const UNINSTALL_SCRIPT: &str = include_str!("../../native/windows/apo/uninstall.ps1");

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
fn run_embedded_installer() -> Result<(), String> {
    verify_embedded_payload()?;
    let package = temporary_package_root();
    if package.exists() {
        fs::remove_dir_all(&package)
            .map_err(|error| format!("failed to reset temporary system-audio package: {error}"))?;
    }
    stage_embedded_package(&package)?;

    let installer = package.join("install.ps1");
    let status = Command::new(windows_powershell())
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&installer)
        .arg("-PackageRoot")
        .arg(&package)
        .status()
        .map_err(|error| {
            format!("failed to launch embedded Windows system-audio installer: {error}")
        });

    let cleanup = fs::remove_dir_all(&package);
    let status = status?;
    if !status.success() {
        return Err(format!(
            "Windows system-audio installer exited with status {}",
            status.code().unwrap_or(-1)
        ));
    }
    if let Err(error) = cleanup {
        return Err(format!(
            "system-audio component installed, but temporary package cleanup failed: {error}"
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn install_windows_audio_component() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        run_embedded_installer()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Windows system-audio installation is unavailable on this platform".into())
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
