mod catalog;
pub mod commands;
mod download;
mod dto;
pub mod manager;
mod storage;

use tauri::AppHandle;

pub use catalog::DEFAULT_MODEL_ID;

pub const AI_RUNTIME_AVAILABLE: bool = false;

pub fn default_model_installed(app: &AppHandle) -> Result<bool, String> {
    let descriptor = catalog::find(DEFAULT_MODEL_ID)?;
    storage::is_installed(app, &descriptor)
}

/// Returns true only when a verified local model exists and this build has a real AI inference backend.
pub fn ai_runtime_ready(app: &AppHandle) -> Result<bool, String> {
    Ok(AI_RUNTIME_AVAILABLE && default_model_installed(app)?)
}
