use super::state::AppState;
use crate::platform::ProcessingController;
use tauri::{AppHandle, State};
use voxveil_types::AiPreferences;

#[tauri::command]
pub async fn set_ai_preferences(
    app: AppHandle,
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
    preferences: AiPreferences,
) -> Result<(), String> {
    preferences.validate()?;
    let mut persisted = crate::config::windows_audio::load(&app)?;
    if preferences.model != state.lock()?.ai.model {
        controller.stop_owned_playback()?;
    }
    persisted.ai = preferences.clone();
    crate::config::windows_audio::save(&app, &persisted)?;
    state.lock()?.ai = preferences;
    Ok(())
}
