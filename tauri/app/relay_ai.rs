use crate::platform::{BackendSnapshot, ProcessingController};
use tauri::{AppHandle, Manager, State};
use voxveil_types::{AiPreferences, ProcessingEngineKind, SystemAiStatus};

pub fn configure(
    app: &AppHandle,
    controller: &ProcessingController,
    prefs: &AiPreferences,
    engine: ProcessingEngineKind,
) -> Result<BackendSnapshot, String> {
    let may_use_ai = prefs.system_wide
        && prefs.model == voxveil_types::AiModel::Spleeter
        && engine != ProcessingEngineKind::Dsp;
    let config = if may_use_ai {
        crate::models::player_config(app, prefs.model).ok()
    } else {
        None
    };
    let status = voxveil_types::relay_ai_status(prefs, engine, config.is_some());
    controller.configure_relay_ai(voxveil_windows_audio::RelayAiOptions {
        config,
        status,
        engine,
        log_path: app
            .path()
            .app_log_dir()
            .ok()
            .map(|directory| directory.join("relay-ai.log")),
    })
}

#[tauri::command]
pub fn get_system_ai_status(controller: State<'_, ProcessingController>) -> SystemAiStatus {
    controller.system_ai_status()
}
