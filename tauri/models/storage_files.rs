use super::catalog::ModelDescriptor;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct FilePaths {
    pub model: PathBuf,
    pub temporary: PathBuf,
}

pub struct ModelPaths {
    pub directory: PathBuf,
    pub files: Vec<FilePaths>,
    pub receipt: PathBuf,
}

pub fn paths(root: &Path, descriptor: &ModelDescriptor) -> ModelPaths {
    let directory = root.join(&descriptor.id);
    ModelPaths {
        files: descriptor
            .files
            .iter()
            .map(|file| FilePaths {
                model: directory.join(&file.file_name),
                temporary: directory.join(format!("{}.download", file.file_name)),
            })
            .collect(),
        receipt: directory.join("install-receipt.txt"),
        directory,
    }
}

pub fn is_installed(paths: &ModelPaths, descriptor: &ModelDescriptor) -> Result<bool, String> {
    if descriptor.files.is_empty() {
        return Ok(false);
    }
    let receipt = match fs::read_to_string(&paths.receipt) {
        Ok(receipt) => receipt,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err("AI model receipt cannot be inspected".to_string()),
    };
    if !receipt
        .lines()
        .any(|line| line == format!("model_id={}", descriptor.id))
    {
        return Ok(false);
    }
    for (file, path) in descriptor.files.iter().zip(&paths.files) {
        let metadata = match fs::metadata(&path.model) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(_) => return Err("AI model file cannot be inspected".to_string()),
        };
        let expected = format!(
            "file={} sha256={} bytes={}",
            file.file_name,
            file.sha256,
            metadata.len()
        );
        if !metadata.is_file()
            || metadata.len() == 0
            || metadata.len() > file.max_bytes
            || !receipt.lines().any(|line| line == expected)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn write_receipt(
    paths: &ModelPaths,
    descriptor: &ModelDescriptor,
    bytes: &[u64],
) -> Result<(), String> {
    if bytes.len() != descriptor.files.len() {
        return Err("AI model receipt is incomplete".to_string());
    }
    let mut receipt = format!(
        "model_id={}\nsource={}\nsource_revision={}\nlicense={}\nuser_consent=true\n",
        descriptor.id, descriptor.source, descriptor.source_revision, descriptor.license
    );
    for (file, bytes) in descriptor.files.iter().zip(bytes) {
        receipt.push_str(&format!(
            "file={} sha256={} bytes={}\n",
            file.file_name, file.sha256, bytes
        ));
    }
    fs::write(&paths.receipt, receipt)
        .map_err(|_| "AI model receipt could not be written".to_string())
}

pub fn verified(paths: &ModelPaths, descriptor: &ModelDescriptor) -> Result<bool, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    if !is_installed(paths, descriptor)? {
        return Ok(false);
    }
    for (path, descriptor) in paths.files.iter().zip(&descriptor.files) {
        let mut file = fs::File::open(&path.model).map_err(|_| "AI model cannot be verified")?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 65536];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|_| "AI model verification was interrupted")?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        if super::transfer::hex(hasher.finalize().as_slice()) != descriptor.sha256 {
            return Ok(false);
        }
    }
    Ok(true)
}
