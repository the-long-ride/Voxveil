use crate::Model;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};
use voxveil_model_api::SeparationError;

#[derive(Deserialize)]
struct Catalog {
    models: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    id: String,
    files: Vec<ModelFile>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelFile {
    file_name: String,
    sha256: String,
    max_bytes: u64,
}

pub(super) fn read_verified(
    model: Model,
    directory: &Path,
) -> Result<Vec<Vec<u8>>, SeparationError> {
    let id = match model {
        Model::Spleeter => "spleeter-2stems-fp16",
        Model::MdxInstHq3 => "mdx-inst-hq3",
    };
    let catalog: Catalog = serde_json::from_str(include_str!("../../../tauri/models/catalog.json"))
        .map_err(|_| SeparationError::ModelUnavailable)?;
    let entry = catalog
        .models
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or(SeparationError::ModelUnavailable)?;
    entry
        .files
        .iter()
        .map(|file| read_file(directory, file))
        .collect()
}

fn read_file(directory: &Path, descriptor: &ModelFile) -> Result<Vec<u8>, SeparationError> {
    let file = File::open(directory.join(&descriptor.file_name))
        .map_err(|_| SeparationError::ModelUnavailable)?;
    let mut bytes = Vec::new();
    file.take(descriptor.max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| SeparationError::ModelUnavailable)?;
    let actual: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if bytes.is_empty() || bytes.len() as u64 > descriptor.max_bytes || actual != descriptor.sha256
    {
        return Err(SeparationError::ModelUnavailable);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inference_receives_only_the_exact_verified_bytes_and_refuses_replacements() {
        let root = std::env::temp_dir().join(format!("voxveil-hash-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("test.onnx");
        std::fs::write(&path, b"abc").unwrap();
        let file = ModelFile {
            file_name: "test.onnx".into(),
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            max_bytes: 3,
        };
        let verified = read_file(&root, &file).unwrap();
        std::fs::write(&path, b"abd").unwrap();
        assert_eq!(
            read_file(&root, &file),
            Err(SeparationError::ModelUnavailable)
        );
        assert_eq!(verified, b"abc");
        assert!(read_verified(Model::Spleeter, &root).is_err());
        std::fs::write(&path, b"abcd").unwrap();
        assert!(read_file(&root, &file).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
