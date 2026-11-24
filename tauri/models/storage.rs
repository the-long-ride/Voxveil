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
    Ok(resolve(app, descriptor)?.installed)
}

pub fn resolve(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<super::location::ModelLocation, String> {
    let resources = app.path().resource_dir().ok().map(|root| root.join("models"));
    super::location::resolve(paths(app, descriptor)?, resources.as_deref(), descriptor)
}

pub fn remove(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<(), String> {
    resolve(app, descriptor)?.ensure_removable()?;
    let paths = paths(app, descriptor)?;
    if paths.directory.exists() {
        fs::remove_dir_all(paths.directory)
            .map_err(|_| "AI model files could not be removed".to_string())?;
    }
    Ok(())
}
