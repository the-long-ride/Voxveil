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
    let previous = state.lock()?.clone();
    if preferences.model != state.lock()?.ai.model {
        controller.stop_owned_playback()?;
    }
    persisted.ai = preferences.clone();
    let changed = super::relay_ai::configure(&app, &controller, &preferences, previous.engine)
        .and_then(|_| crate::config::windows_audio::save(&app, &persisted));
    if let Err(error) = changed {
        let _ = super::relay_ai::configure(&app, &controller, &previous.ai, previous.engine);
        return Err(error);
    }
    state.lock()?.ai = preferences;
    Ok(())
}
