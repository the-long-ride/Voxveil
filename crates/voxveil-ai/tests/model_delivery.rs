#![allow(dead_code)]
#[path = "../../../tauri/models/catalog.rs"]
mod catalog;
#[path = "../../../tauri/models/storage_files.rs"]
mod storage_files;
#[path = "../../../tauri/models/transfer.rs"]
mod transfer;

use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!(
                "model-delivery-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn single_and_multi_file_installations_require_every_file_and_matching_receipt() {
    for id in [catalog::DEFAULT_MODEL_ID, "mdx-inst-hq3"] {
        let root = Directory::new();
        let mut model = catalog::find(id).unwrap();
        for file in &mut model.files {
            file.max_bytes = 8;
        }
        let paths = storage_files::paths(&root.0, &model);
        assert!(!storage_files::is_installed(&paths, &model).unwrap());
        fs::create_dir_all(&paths.directory).unwrap();
        for path in &paths.files {
            fs::write(&path.model, b"test").unwrap();
            assert_ne!(path.model, path.temporary);
        }
        assert!(!storage_files::is_installed(&paths, &model).unwrap());
        let sizes = vec![4; model.files.len()];
        storage_files::write_receipt(&paths, &model, &sizes).unwrap();
        assert!(storage_files::is_installed(&paths, &model).unwrap());
        let last = paths.files.last().unwrap();
        fs::remove_file(&last.model).unwrap();
        assert!(!storage_files::is_installed(&paths, &model).unwrap());
        fs::write(&last.model, b"123456789").unwrap();
        assert!(!storage_files::is_installed(&paths, &model).unwrap());
        fs::write(&last.model, b"").unwrap();
        assert!(!storage_files::is_installed(&paths, &model).unwrap());
        fs::write(&last.model, b"test").unwrap();
        model.files.last_mut().unwrap().sha256 = "0".repeat(64);
        assert!(!storage_files::is_installed(&paths, &model).unwrap());
        assert!(storage_files::write_receipt(&paths, &model, &[]).is_err());
    }
    assert!(catalog::find("htdemucs-ft-vocals-fp16").is_err());
}

#[test]
fn transfers_enforce_hash_size_nonempty_and_io_errors() {
    let mut file = catalog::find("mdx-inst-hq3").unwrap().files.remove(0);
    file.sha256 = transfer::hex(Sha256::digest(b"abc").as_slice());
    file.max_bytes = 3;
    let mut output = Vec::new();
    let mut progress = 0;
    assert_eq!(
        transfer::copy_verified(&mut &b"abc"[..], &mut output, &file, |n| progress = n).unwrap(),
        3
    );
    assert_eq!(output, b"abc");
    assert_eq!(progress, 3);
    for bytes in [&b"abcd"[..], &b"abd"[..], &b""[..]] {
        assert!(transfer::copy_verified(&mut &*bytes, &mut Vec::new(), &file, |_| {}).is_err());
    }
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("interrupted"))
        }
    }
    assert!(
        transfer::copy_verified(&mut Broken, &mut Vec::new(), &file, |_| {})
            .unwrap_err()
            .contains("interrupted")
    );
    assert!(
        transfer::copy_verified(&mut &b"abc"[..], &mut &mut [0_u8; 0][..], &file, |_| {}).is_err()
    );
}
