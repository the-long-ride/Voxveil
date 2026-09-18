use std::os::windows::process::CommandExt;
use std::path::Path;

use super::powershell_single_quoted;

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

pub(super) fn launch_system_audio_installer(
    script: &Path,
    descriptor: &Path,
    descriptor_sha256: &str,
) -> Result<InstallerLaunchOutcome, String> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script = powershell_single_quoted(&script.to_string_lossy());
    let descriptor = powershell_single_quoted(&descriptor.to_string_lossy());
    let descriptor_sha256 = powershell_single_quoted(descriptor_sha256);
    let launch = format!(
        r#"$ErrorActionPreference='Stop'; $script='{script}'; $descriptor='{descriptor}'; $descriptorSha256='{descriptor_sha256}'; $scriptArg='"' + $script + '"'; $descriptorArg='"' + $descriptor + '"'; try {{ $process=Start-Process -FilePath 'powershell.exe' -Verb RunAs -Wait -PassThru -ArgumentList @('-NoLogo','-NoProfile','-ExecutionPolicy','Bypass','-File',$scriptArg,'-EndpointDescriptor',$descriptorArg,'-EndpointDescriptorSha256',$descriptorSha256); exit $process.ExitCode }} catch {{ Write-Error $_; exit 1 }}"#,
    );
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &launch])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|error| format!("failed to open the system-audio installer: {error}"))?;
    installer_launch_outcome(status.code())
}
