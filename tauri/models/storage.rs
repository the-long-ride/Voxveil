use super::{
    catalog::ModelDescriptor,
    storage_files::{self, ModelPaths},
};
use std::fs;
use tauri::{AppHandle, Manager};
#[path = "retired.rs"]
mod retired;

pub fn paths(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<ModelPaths, String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "application data directory is unavailable".to_string())?
        .join("models");
    retired::cleanup(&root)?;
    Ok(storage_files::paths(&root, descriptor))
}

pub fn is_installed(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<bool, String> {
    storage_files::is_installed(&paths(app, descriptor)?, descriptor)
}

pub fn remove(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<(), String> {
    let paths = paths(app, descriptor)?;
    if paths.directory.exists() {
        fs::remove_dir_all(paths.directory)
            .map_err(|_| "AI model files could not be removed".to_string())?;
    }
    Ok(())
}
