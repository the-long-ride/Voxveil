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
fn empty_descriptors_and_resource_io_errors_cannot_be_treated_as_installed() {
    let fixture = Fixture::new();
    let mut model = descriptor("mdx-inst-hq3");
    let paths = fixture.paths("resources", &model);
    write_files(&paths);
    model.files.clear();
    assert!(!fixture.resolve(&model).installed);
    model = descriptor("mdx-inst-hq3");
    fs::write(fixture.0.join("not-a-directory"), b"abc").unwrap();
    assert!(location::resolve(
        fixture.paths("downloads", &model),
        Some(Path::new(&fixture.0.join("not-a-directory"))),
        &model
    )
    .is_err());
}
