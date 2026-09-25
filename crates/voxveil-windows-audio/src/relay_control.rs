use std::env;
use std::path::PathBuf;
use std::process::Command;

fn apo_install_state_exists() -> Result<bool, String> {
    let path = system_audio_directory().join("install-state.json");
    match std::fs::metadata(&path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "failed to inspect Voxveil APO install state at {}: {error}",
            path.display()
        )),
    }
}

pub(super) fn control_executable_for_installed_apo() -> Result<Option<PathBuf>, String> {
    let control = control_executable();
    if control.is_none() && apo_install_state_exists()? {
        return Err(
            "Voxveil APO install state exists but its control component is unavailable; load state cannot be verified"
                .to_string(),
        );
    }
    Ok(control)
}

pub(super) fn loaded_apo_instances() -> Result<u32, String> {
    let Some(control) = control_executable_for_installed_apo()? else {
        return Ok(0);
    };
    let status = run_control(&control, &["status"])?;
    parse_loaded_instances_required(&status)
}

pub(super) fn system_audio_directory() -> PathBuf {
    if let Ok(path) = env::var("VOXVEIL_SYSTEM_AUDIO_DIR") {
        return PathBuf::from(path);
    }
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("system-audio")))
        .unwrap_or_else(|| PathBuf::from("system-audio"))
}

pub(super) fn control_executable() -> Option<PathBuf> {
    if let Ok(path) = env::var("VOXVEIL_CONTROL_EXE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let directory = env::current_exe().ok()?.parent()?.to_path_buf();
    [
        directory.join("voxveil-control.exe"),
        directory.join("system-audio").join("voxveil-control.exe"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

pub(super) fn run_control(control: &PathBuf, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(control);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command
        .output()
        .map_err(|error| format!("failed to run {}: {error}", control.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{} exited with {}", control.display(), output.status)
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn parse_loaded_instances(status: &str) -> Option<u32> {
    status
        .split_whitespace()
        .find_map(|part| part.strip_prefix("loaded=")?.parse().ok())
}

pub(super) fn parse_loaded_instances_required(status: &str) -> Result<u32, String> {
    parse_loaded_instances(status).ok_or_else(|| {
        "Voxveil APO control loaded=<count> status field is missing or invalid".to_string()
    })
}
