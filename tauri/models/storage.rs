use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use super::catalog::ModelDescriptor;

pub struct ModelPaths {
    pub directory: PathBuf,
    pub model: PathBuf,
    pub temporary: PathBuf,
    pub receipt: PathBuf,
}

pub fn paths(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<ModelPaths, String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "application data directory is unavailable".to_string())?
        .join("models")
        .join(&descriptor.id);
    Ok(ModelPaths {
        model: root.join(&descriptor.file_name),
        temporary: root.join(format!("{}.download", descriptor.file_name)),
        receipt: root.join("install-receipt.txt"),
        directory: root,
    })
}

pub fn is_installed(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<bool, String> {
    let paths = paths(app, descriptor)?;
    let metadata = match fs::metadata(&paths.model) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err("AI model file cannot be inspected".to_string()),
    };
    if !metadata.is_file() || metadata.len() > descriptor.max_bytes {
        return Ok(false);
    }
    let receipt = match fs::read_to_string(&paths.receipt) {
        Ok(receipt) => receipt,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err("AI model receipt cannot be inspected".to_string()),
    };
    let expected = format!("sha256={}", descriptor.sha256);
    Ok(receipt.lines().any(|line| line == expected))
}

pub fn write_receipt(
    paths: &ModelPaths,
    descriptor: &ModelDescriptor,
    bytes: u64,
) -> Result<(), String> {
    let receipt = format!(
        "model_id={}\nsource={}\nsource_revision={}\nlicense={}\nsha256={}\nbytes={}\nuser_consent=true\n",
        descriptor.id,
        descriptor.source,
        descriptor.source_revision,
        descriptor.license,
        descriptor.sha256,
        bytes,
    );
    fs::write(&paths.receipt, receipt)
        .map_err(|_| "AI model receipt could not be written".to_string())
}

pub fn remove(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<(), String> {
    let paths = paths(app, descriptor)?;
    if paths.directory.exists() {
        fs::remove_dir_all(paths.directory)
            .map_err(|_| "AI model files could not be removed".to_string())?;
    }
    Ok(())
}
