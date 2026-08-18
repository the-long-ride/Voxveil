use std::path::{Path, PathBuf};

fn installer_path_for_exe(exe: &Path) -> Result<PathBuf, String> {
    let directory = exe
        .parent()
        .ok_or_else(|| "Voxveil executable has no parent directory".to_string())?;
    Ok(directory.join("system-audio").join("install.ps1"))
}

#[cfg(target_os = "windows")]
fn packaged_installer_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("failed to locate Voxveil executable: {error}"))?;
    installer_path_for_exe(&exe)
}

#[tauri::command]
pub fn install_windows_audio_component() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let installer = packaged_installer_path()?;
        if !installer.is_file() {
            return Err(format!(
                "Windows system-audio installer is missing: {}",
                installer.display()
            ));
        }

        let status = std::process::Command::new("powershell.exe")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&installer)
            .status()
            .map_err(|error| format!("failed to launch Windows system-audio installer: {error}"))?;
        if !status.success() {
            return Err(format!(
                "Windows system-audio installer exited with status {}",
                status.code().unwrap_or(-1)
            ));
        }
        Ok(())
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
    fn installer_is_sibling_of_packaged_executable() {
        let exe = Path::new(r"C:\Voxveil\voxveil.exe");
        assert_eq!(
            installer_path_for_exe(exe).unwrap(),
            PathBuf::from(r"C:\Voxveil\system-audio\install.ps1")
        );
    }
}
