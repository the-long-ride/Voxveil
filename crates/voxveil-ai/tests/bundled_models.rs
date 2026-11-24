#![allow(dead_code)]
#[path = "../../../tauri/models/catalog.rs"]
mod catalog;
#[path = "../../../tauri/models/location.rs"]
mod location;
#[path = "../../../tauri/models/storage_files.rs"]
mod storage_files;
#[path = "../../../tauri/models/transfer.rs"]
mod transfer;

use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
#[cfg(windows)]
use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!(
                "bundled-models-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn paths(
        &self,
        directory: &str,
        model: &catalog::ModelDescriptor,
    ) -> storage_files::ModelPaths {
        storage_files::paths(&self.0.join(directory), model)
    }
    fn resolve(&self, model: &catalog::ModelDescriptor) -> location::ModelLocation {
        location::resolve(
            self.paths("downloads", model),
            Some(&self.0.join("resources")),
            model,
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn descriptor(id: &str) -> catalog::ModelDescriptor {
    let mut model = catalog::find(id).unwrap();
    for file in &mut model.files {
        file.sha256 = transfer::hex(Sha256::digest(b"abc").as_slice());
        file.max_bytes = 3;
    }
    model
}

fn write_files(paths: &storage_files::ModelPaths) {
    fs::create_dir_all(&paths.directory).unwrap();
    for path in &paths.files {
        fs::write(&path.model, b"abc").unwrap();
    }
}

fn write_download(paths: &storage_files::ModelPaths, model: &catalog::ModelDescriptor) {
    write_files(paths);
    storage_files::write_receipt(paths, model, &vec![3; model.files.len()]).unwrap();
}

#[test]
fn bundled_models_are_ready_without_receipts_or_downloads_and_cannot_be_removed() {
    assert_eq!(catalog::DEFAULT_MODEL_ID, "mdx-inst-hq3");
    for id in ["spleeter-2stems-fp16", "mdx-inst-hq3"] {
        let fixture = Fixture::new();
        let model = descriptor(id);
        assert!(model.bundled_by_default);
        let resources = fixture.paths("resources", &model);
        write_files(&resources);
        let resolved = fixture.resolve(&model);
        assert!(resolved.installed && resolved.bundled);
        assert_eq!(resolved.paths.directory, resources.directory);
        assert!(!resources.receipt.exists());
        assert!(!fixture.0.join("downloads").exists());
        assert!(resolved.ensure_removable().is_err());
        assert!(storage_files::verified_files(&resources, &model).unwrap());
        write_download(&fixture.paths("downloads", &model), &model);
        assert!(fixture.resolve(&model).bundled);
    }
}

#[test]
fn absent_resources_use_verified_downloads_and_allow_removal() {
    let fixture = Fixture::new();
    let model = descriptor("spleeter-2stems-fp16");
    assert!(!fixture.resolve(&model).installed);
    let downloaded = fixture.paths("downloads", &model);
    write_download(&downloaded, &model);
    for resources in [None, Some(fixture.0.join("absent"))] {
        let resolved = location::resolve(
            fixture.paths("downloads", &model),
            resources.as_deref(),
            &model,
        )
        .unwrap();
        assert!(resolved.installed && !resolved.bundled);
        assert_eq!(resolved.paths.directory, downloaded.directory);
        assert!(resolved.ensure_removable().is_ok());
    }
    fs::write(&downloaded.files[0].model, b"abd").unwrap();
    assert!(!fixture.resolve(&model).installed);
}

#[test]
fn corrupt_empty_oversized_or_partial_bundles_never_enable_ai_and_can_fall_back() {
    for id in ["spleeter-2stems-fp16", "mdx-inst-hq3"] {
        let fixture = Fixture::new();
        let model = descriptor(id);
        let resources = fixture.paths("resources", &model);
        for invalid in [
            Some(b"abd".as_slice()),
            Some(b"".as_slice()),
            Some(b"abcd".as_slice()),
            None,
        ] {
            write_files(&resources);
            let last = &resources.files.last().unwrap().model;
            if let Some(bytes) = invalid {
                fs::write(last, bytes).unwrap();
            } else {
                fs::remove_file(last).unwrap();
            }
            assert!(!fixture.resolve(&model).installed);
        }
        write_download(&fixture.paths("downloads", &model), &model);
        let resolved = fixture.resolve(&model);
        assert!(resolved.installed && !resolved.bundled);
        assert_eq!(
            resolved.paths.directory,
            fixture.paths("downloads", &model).directory
        );
    }
}

#[test]
fn empty_descriptors_cannot_be_treated_as_installed() {
    let fixture = Fixture::new();
    let mut model = descriptor("mdx-inst-hq3");
    let paths = fixture.paths("resources", &model);
    write_files(&paths);
    model.files.clear();
    assert!(!fixture.resolve(&model).installed);
}

#[test]
fn invalid_resource_paths_fall_back_to_downloads_without_skipping_verification() {
    let fixture = Fixture::new();
    let model = descriptor("mdx-inst-hq3");
    let resources = fixture.0.join("not-a-directory");
    fs::write(&resources, b"abc").unwrap();
    let bundled = storage_files::paths(&resources, &model);
    assert!(!matches!(
        storage_files::verified_files(&bundled, &model),
        Ok(true)
    ));
    let resolve = || {
        location::resolve(
            fixture.paths("downloads", &model),
            Some(Path::new(&resources)),
            &model,
        )
    };
    let missing = resolve().unwrap();
    assert!(!missing.installed && !missing.bundled);

    let downloaded = fixture.paths("downloads", &model);
    write_download(&downloaded, &model);
    let fallback = resolve().unwrap();
    assert!(fallback.installed && !fallback.bundled);
    assert_eq!(fallback.paths.directory, downloaded.directory);

    fs::write(&downloaded.files[0].model, b"abd").unwrap();
    assert!(!resolve().unwrap().installed);
    fs::remove_file(&downloaded.receipt).unwrap();
    fs::create_dir(&downloaded.receipt).unwrap();
    assert!(resolve().is_err());
}

#[cfg(windows)]
#[test]
fn bundled_file_sharing_violations_fall_back_to_verified_downloads() {
    for id in ["spleeter-2stems-fp16", "mdx-inst-hq3"] {
        let fixture = Fixture::new();
        let model = descriptor(id);
        let bundled = fixture.paths("resources", &model);
        write_files(&bundled);
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&bundled.files[0].model)
            .unwrap();
        let error = fs::File::open(&bundled.files[0].model).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(32));
        assert!(storage_files::verified_files(&bundled, &model).is_err());
        assert!(!fixture.resolve(&model).installed);

        let downloaded = fixture.paths("downloads", &model);
        write_download(&downloaded, &model);
        let fallback = fixture.resolve(&model);
        assert!(fallback.installed && !fallback.bundled);
        assert_eq!(fallback.paths.directory, downloaded.directory);
        fs::write(&downloaded.files[0].model, b"abd").unwrap();
        assert!(!fixture.resolve(&model).installed);

        drop(locked);
        let restored = fixture.resolve(&model);
        assert!(restored.installed && restored.bundled);
    }
}
