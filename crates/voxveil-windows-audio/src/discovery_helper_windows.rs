use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::topology::windows_system_directory;

use super::windows::{InputEndpoint, ResolvedEndpoint};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub(super) fn run_fallback_helper(
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
        return Err(format!(
            "Windows PowerShell was not found at {}.",
            powershell.display()
        ));
    }

    let mut child = Command::new(&powershell)
        .args(["-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
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
