use tauri::{AppHandle, State, Manager};

use super::{catalog, download, dto::AiModelStatusDto, manager::ModelManager, storage};

fn status(app: &AppHandle, model_id: &str) -> Result<AiModelStatusDto, String> {
    let descriptor = catalog::find(model_id)?;
    let location = storage::resolve(app, &descriptor)?;
    Ok(AiModelStatusDto::from_descriptor(&descriptor, location.installed, location.bundled))
}

fn refresh_relay(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<crate::app::state::AppState>();
    let current = state.lock()?.clone();
    crate::app::relay_ai::configure(app, &app.state::<crate::platform::ProcessingController>(), &current.ai, current.engine)?;
    Ok(())
}

#[tauri::command]
pub fn get_ai_model_status(
    app: AppHandle,
    model_id: Option<String>,
) -> Result<AiModelStatusDto, String> {
    let selected = crate::config::windows_audio::load(&app)?.ai.model;
    status(&app, model_id.as_deref().unwrap_or(selected.id()))
}

#[tauri::command]
pub async fn install_ai_model(
    app: AppHandle,
    manager: State<'_, ModelManager>,
    model_id: String,
    accepted_terms: bool,
) -> Result<AiModelStatusDto, String> {
    if !accepted_terms {
        return Err("explicit model-download consent is required".to_string());
    }
    let descriptor = catalog::find(&model_id)?;
    let gate = manager.gate();
    let task_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .map_err(|_| "AI model manager lock is poisoned".to_string())?;
        download::install(&task_app, &descriptor)
    })
    .await
    .map_err(|_| "AI model download task failed".to_string())??;
    refresh_relay(&app)?;
    status(&app, &model_id)
}

#[tauri::command]
pub async fn remove_ai_model(
    app: AppHandle,
    controller: State<'_, crate::platform::ProcessingController>,
    manager: State<'_, ModelManager>,
    model_id: String,
) -> Result<AiModelStatusDto, String> {
    let descriptor = catalog::find(&model_id)?;
    storage::resolve(&app, &descriptor)?.ensure_removable()?;
    controller.stop_owned_playback_for_model(&model_id)?;
    let gate = manager.gate();
    let task_app = app.clone();
    let task_descriptor = descriptor.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .lock()
            .map_err(|_| "AI model manager lock is poisoned".to_string())?;
        storage::remove(&task_app, &task_descriptor)
    })
    .await
    .map_err(|_| "AI model removal task failed".to_string())??;
    refresh_relay(&app)?;
    status(&app, &model_id)
}
