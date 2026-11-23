use std::{fs, path::Path};

const RETIRED: &[&str] = &["htdemucs-ft-vocals-fp16"];

pub fn cleanup(root: &Path) -> Result<(), String> {
    for id in RETIRED {
        let directory = root.join(id);
        match fs::symlink_metadata(&directory) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("retired AI model cannot be inspected".into()),
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Err("retired AI model directory is not a regular directory".into());
            }
            Ok(_) => fs::remove_dir_all(directory)
                .map_err(|_| "retired AI model files could not be removed")?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upgrade_removes_only_the_known_retired_directory_and_is_idempotent() {
        let root = std::env::temp_dir().join(format!("voxveil-retired-{}", std::process::id()));
        let old = root.join(RETIRED[0]);
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("weights.onnx"), b"retired download").unwrap();
        let current = root.join("spleeter-2stems-fp16");
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("vocals.onnx"), b"keep").unwrap();
        cleanup(&root).unwrap();
        cleanup(&root).unwrap();
        assert!(!old.exists());
        assert_eq!(fs::read(current.join("vocals.onnx")).unwrap(), b"keep");
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn refuses_a_link_instead_of_following_it() {
        let root =
            std::env::temp_dir().join(format!("voxveil-retired-link-{}", std::process::id()));
        fs::create_dir_all(root.join("keep")).unwrap();
        fs::write(root.join("keep/file"), b"keep").unwrap();
        std::os::unix::fs::symlink(root.join("keep"), root.join(RETIRED[0])).unwrap();
        assert!(cleanup(&root).is_err());
        assert!(root.join("keep/file").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
