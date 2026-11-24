mod catalog;
pub mod commands;
mod download;
mod dto;
mod location;
pub mod manager;
mod storage;
mod storage_files;
mod transfer;

use tauri::AppHandle;

pub use catalog::DEFAULT_MODEL_ID;

pub fn runtime_path() -> Option<std::path::PathBuf> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    Some(
        std::env::current_exe()
            .ok()?
            .parent()?
            .join("onnxruntime.dll"),
    )
}

pub fn runtime_available() -> bool {
    runtime_path().is_some_and(|path| voxveil_ai::runtime_available(&path))
}

pub fn default_model_installed(app: &AppHandle) -> Result<bool, String> {
    let descriptor = catalog::find(DEFAULT_MODEL_ID)?;
    storage::is_installed(app, &descriptor)
}

/// Returns true only when a verified local model exists and this build has a real AI inference backend.
pub fn ai_runtime_ready(app: &AppHandle) -> Result<bool, String> {
    let selected = crate::config::windows_audio::load(app)?.ai.model;
    let descriptor = catalog::find(selected.id())?;
    Ok(runtime_available() && storage::resolve(app, &descriptor)?.installed)
}

pub fn player_config(
    app: &AppHandle,
    model: voxveil_types::AiModel,
) -> Result<voxveil_ai::player::AiPlaybackConfig, String> {
    let descriptor = catalog::find(model.id())?;
    let location = storage::resolve(app, &descriptor)?;
    if !runtime_available() || !location.installed {
        return Err("AI is unavailable: selected model files or app-owned ONNX Runtime are missing or invalid".into());
    }
    Ok(voxveil_ai::player::AiPlaybackConfig {
        model: match model {
            voxveil_types::AiModel::Spleeter => voxveil_ai::Model::Spleeter,
            voxveil_types::AiModel::MdxInstHq3 => voxveil_ai::Model::MdxInstHq3,
        },
        directory: location.paths.directory,
        runtime: runtime_path().ok_or("AI runtime is unavailable on this platform")?,
    })
}
