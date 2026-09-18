use std::os::windows::process::CommandExt;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::powershell_single_quoted;

const TRUSTED_SYSTEM_AUDIO_INSTALLER: &[u8] =
    include_bytes!("../../scripts/windows/install-system-audio-component.ps1");

const TRUSTED_DISCOVERY_SHA256: Option<&str> = option_env!("VOXVEIL_DISCOVERY_SHA256");
const TRUSTED_CONTROL_SHA256: Option<&str> = option_env!("VOXVEIL_CONTROL_SHA256");
const TRUSTED_CONTROL_DLL_SHA256: Option<&str> = option_env!("VOXVEIL_CONTROL_DLL_SHA256");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InstallerLaunchOutcome {
    Completed,
    RebootRequired,
}

pub(super) fn installer_launch_outcome(
    exit_code: Option<i32>,
) -> Result<InstallerLaunchOutcome, String> {
    match exit_code {
        Some(0) => Ok(InstallerLaunchOutcome::Completed),
        Some(3010) => Ok(InstallerLaunchOutcome::RebootRequired),
        _ => Err("The system-audio installer was cancelled or exited with an error.".into()),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn verify_trusted_installer(script: &Path) -> Result<String, String> {
    let expected = sha256_hex(TRUSTED_SYSTEM_AUDIO_INSTALLER);
    let actual_bytes = std::fs::read(script)
        .map_err(|error| format!("failed to read bundled system-audio installer: {error}"))?;
    let actual = sha256_hex(&actual_bytes);
    if actual != expected {
        return Err("Bundled system-audio installer failed integrity verification.".into());
    }
    Ok(expected)
}

fn embedded_sha256(value: Option<&'static str>, description: &str) -> Result<String, String> {
    let value = value.ok_or_else(|| {
        format!("Voxveil was built without the trusted {description} SHA-256.")
    })?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("Voxveil contains an invalid trusted {description} SHA-256."));
    }
    Ok(value.to_ascii_lowercase())
}

fn verify_trusted_packaged_file(
    path: &Path,
    expected_sha256: &str,
    description: &str,
) -> Result<(), String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read packaged {description}: {error}"))?;
    if sha256_hex(&bytes) != expected_sha256 {
        return Err(format!("Packaged {description} failed integrity verification."));
    }
    Ok(())
}

pub(super) fn launch_system_audio_installer(
    script: &Path,
    descriptor: &Path,
    descriptor_sha256: &str,
) -> Result<InstallerLaunchOutcome, String> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script_sha256 = verify_trusted_installer(script)?;
    let system_audio_dir = script
        .parent()
        .ok_or_else(|| "Bundled system-audio installer has no parent directory.".to_string())?;
    let discovery_sha256 = embedded_sha256(TRUSTED_DISCOVERY_SHA256, "endpoint discovery helper")?;
    let control_sha256 = embedded_sha256(TRUSTED_CONTROL_SHA256, "control executable")?;
    let control_dll_sha256 = embedded_sha256(TRUSTED_CONTROL_DLL_SHA256, "control DLL")?;
    verify_trusted_packaged_file(
        &system_audio_dir.join("discover-system-audio-endpoints.ps1"),
        &discovery_sha256,
        "endpoint discovery helper",
    )?;
    verify_trusted_packaged_file(
        &system_audio_dir.join("voxveil-control.exe"),
        &control_sha256,
        "control executable",
    )?;
    verify_trusted_packaged_file(
        &system_audio_dir.join("VoxveilControl.dll"),
        &control_dll_sha256,
        "control DLL",
    )?;
    let script = powershell_single_quoted(&script.to_string_lossy());
    let descriptor = powershell_single_quoted(&descriptor.to_string_lossy());
    let descriptor_sha256 = powershell_single_quoted(descriptor_sha256);
    let script_sha256 = powershell_single_quoted(&script_sha256);
    let launch = format!(
        r#"$ErrorActionPreference='Stop'; $script='{script}'; $descriptor='{descriptor}'; $descriptorSha256='{descriptor_sha256}'; $scriptSha256='{script_sha256}'; $scriptB64=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($script)); $descriptorB64=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($descriptor)); $elevatedCommand="`$ErrorActionPreference='Stop'; `$script=[Text.Encoding]::Unicode.GetString([Convert]::FromBase64String('$scriptB64')); `$descriptor=[Text.Encoding]::Unicode.GetString([Convert]::FromBase64String('$descriptorB64')); `$actualScriptSha256=(Get-FileHash -LiteralPath `$script -Algorithm SHA256).Hash.ToLowerInvariant(); if (`$actualScriptSha256 -ne '$scriptSha256') {{ Write-Error 'Bundled system-audio installer integrity check failed.'; exit 1 }}; & `$script -EndpointDescriptor `$descriptor -EndpointDescriptorSha256 '$descriptorSha256' -DiscoveryHelperSha256 '$discovery_sha256' -ControlHelperSha256 '$control_sha256' -ControlDllSha256 '$control_dll_sha256'; exit `$LASTEXITCODE"; $encodedCommand=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($elevatedCommand)); try {{ $process=Start-Process -FilePath 'powershell.exe' -Verb RunAs -Wait -PassThru -ArgumentList @('-NoLogo','-NoProfile','-ExecutionPolicy','Bypass','-EncodedCommand',$encodedCommand); exit $process.ExitCode }} catch {{ Write-Error $_; exit 1 }}"#,
    );
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &launch])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|error| format!("failed to open the system-audio installer: {error}"))?;
    installer_launch_outcome(status.code())
}
