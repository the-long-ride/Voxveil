use super::{
    catalog::ModelDescriptor,
    storage_files::{self, ModelPaths},
};
use std::path::Path;

pub struct ModelLocation {
    pub paths: ModelPaths,
    pub installed: bool,
    pub bundled: bool,
}

impl ModelLocation {
    pub fn ensure_removable(&self) -> Result<(), String> {
        if self.bundled {
            Err("Bundled AI models cannot be removed; they are part of the application".into())
        } else {
            Ok(())
        }
    }
}

pub fn resolve(
    downloaded: ModelPaths,
    resources: Option<&Path>,
    descriptor: &ModelDescriptor,
) -> Result<ModelLocation, String> {
    if let Some(root) = resources.filter(|_| descriptor.bundled_by_default) {
        let paths = storage_files::paths(root, descriptor);
        if storage_files::verified_files(&paths, descriptor)? {
            return Ok(ModelLocation {
                paths,
                installed: true,
                bundled: true,
            });
        }
    }
    let installed = storage_files::verified(&downloaded, descriptor)?;
    Ok(ModelLocation {
        paths: downloaded,
        installed,
        bundled: false,
    })
}
