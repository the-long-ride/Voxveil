use std::{env, fs, path::PathBuf};

fn stage_embedded_windows_audio_payload() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let generated = manifest_dir.join("generated-system-audio");
    fs::create_dir_all(&generated).expect("create generated system-audio directory");

    let edition = env::var("VOXVEIL_EDITION").unwrap_or_else(|_| "standard".to_string());
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let native_out = manifest_dir.join("../native/windows/apo/out/Release");

    for name in ["VoxveilApo.dll", "VoxveilApoCheck.exe"] {
        let destination = generated.join(name);
        if target_os == "windows" && edition == "pro-system" {
            let source = native_out.join(name);
            if !source.is_file() {
                panic!(
                    "pro-system Windows build requires native APO payload: {}",
                    source.display()
                );
            }
            fs::copy(&source, &destination).unwrap_or_else(|error| {
                panic!(
                    "failed to stage embedded APO payload {} -> {}: {error}",
                    source.display(),
                    destination.display()
                )
            });
        } else {
            fs::write(&destination, []).unwrap_or_else(|error| {
                panic!("failed to create placeholder {}: {error}", destination.display())
            });
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=VOXVEIL_EDITION");
    println!("cargo:rerun-if-changed=../native/windows/apo/out/Release/VoxveilApo.dll");
    println!("cargo:rerun-if-changed=../native/windows/apo/out/Release/VoxveilApoCheck.exe");
    println!("cargo:rerun-if-changed=../native/windows/apo/install.ps1");
    println!("cargo:rerun-if-changed=../native/windows/apo/uninstall.ps1");
    stage_embedded_windows_audio_payload();
    tauri_build::build();
}
