use tauri::{AppHandle, State};

use super::dto::AppViewState;
use super::state::AppState;
use crate::platform::ProcessingController;

const WINDOWS_SOUND_SETTINGS_URI: &str = "ms-settings:sound";
const VB_CABLE_DOWNLOAD_URL: &str = "https://vb-audio.com/Cable/";

#[tauri::command]
pub async fn set_physical_audio_output(
    app: AppHandle,
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
    endpoint_id: String,
) -> Result<AppViewState, String> {
    let mut preferences = crate::config::windows_audio::load(&app)?;
    let previous_endpoint_id = preferences.physical_output_endpoint_id.clone();
    let snapshot = controller.set_physical_output(Some(endpoint_id.clone()))?;

    preferences.physical_output_endpoint_id = Some(endpoint_id);
    if let Err(error) = crate::config::windows_audio::save(&app, &preferences) {
        // Keep the live route, AppState and persisted preference consistent when
        // storage fails. Restoring a route stops processing, so the rollback
        // snapshot must be applied even though this command returns an error.
        let (rollback_snapshot, rollback_error) =
            match controller.set_physical_output(previous_endpoint_id) {
                Ok(snapshot) => (snapshot, None),
                Err(rollback_error) => (controller.snapshot(), Some(rollback_error)),
            };
        state.lock()?.apply_backend(&rollback_snapshot);
        return Err(match rollback_error {
            Some(rollback_error) => {
                format!("{error}; failed to restore previous physical output: {rollback_error}")
            }
            None => error,
        });
    }

    let mut current = state.lock()?;
    current.apply_backend(&snapshot);
    Ok(current.clone())
}

#[cfg(target_os = "windows")]
fn open_with_explorer(target: &str, description: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let system_directory = voxveil_windows_audio::windows_system_directory()?;
    let windows_directory = system_directory
        .parent()
        .ok_or_else(|| "Windows system directory has no parent Windows directory.".to_string())?;
    let explorer = windows_directory.join("explorer.exe");
    if !explorer.is_file() {
        return Err(format!("Windows Explorer was not found at {}.", explorer.display()));
    }
    std::process::Command::new(explorer)
        .arg(target)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open {description}: {error}"))
}

#[tauri::command]
pub async fn open_windows_sound_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        return open_with_explorer(WINDOWS_SOUND_SETTINGS_URI, "Windows Sound settings");
    }
    #[cfg(not(target_os = "windows"))]
    Err("Windows Sound settings are unavailable on this platform".into())
}

#[tauri::command]
pub async fn open_vb_cable_download() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        return open_with_explorer(VB_CABLE_DOWNLOAD_URL, "the official VB-CABLE website");
    }
    #[cfg(not(target_os = "windows"))]
    Err("VB-CABLE onboarding is available only on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_targets_are_fixed_and_official() {
        assert_eq!(WINDOWS_SOUND_SETTINGS_URI, "ms-settings:sound");
        assert_eq!(VB_CABLE_DOWNLOAD_URL, "https://vb-audio.com/Cable/");
    }
}
