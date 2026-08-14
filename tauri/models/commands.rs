use tauri::{AppHandle, State};

use super::{catalog, download, dto::AiModelStatusDto, manager::ModelManager, storage};

fn status(app: &AppHandle, model_id: &str) -> Result<AiModelStatusDto, String> {
    let descriptor = catalog::find(model_id)?;
    let installed = storage::is_installed(app, &descriptor)?;
    Ok(AiModelStatusDto::from_descriptor(&descriptor, installed))
}

#[tauri::command]
pub fn get_ai_model_status(app: AppHandle) -> Result<AiModelStatusDto, String> {
    status(&app, catalog::DEFAULT_MODEL_ID)
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
    status(&app, &model_id)
}

#[tauri::command]
pub async fn remove_ai_model(
    app: AppHandle,
    manager: State<'_, ModelManager>,
    model_id: String,
) -> Result<AiModelStatusDto, String> {
    let descriptor = catalog::find(&model_id)?;
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
    status(&app, &model_id)
}
