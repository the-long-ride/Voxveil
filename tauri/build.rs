use std::{env, fs, path::PathBuf};

fn stage_embedded_windows_audio_payload() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let generated = manifest_dir.join("generated-system-audio");
    fs::create_dir_all(&generated).expect("create generated system-audio directory");

    let edition = env::var("VOXVEIL_EDITION").unwrap_or_else(|_| "standard".to_string());
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let native_out = manifest_dir.join("../native/windows/apo/out/Release");

    for name in [
        "VoxveilApo.dll",
        "VoxveilApoCheck.exe",
        "VoxveilApoTarget.exe",
    ] {
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
                panic!(
                    "failed to create placeholder {}: {error}",
                    destination.display()
                )
            });
        }
    }

    fs::write(
        out_dir.join("voxveil-embedded-system-audio.txt"),
        format!("target_os={target_os}\nedition={edition}\n"),
    )
    .expect("write embedded system-audio build marker");
}

fn main() {
    println!("cargo:rerun-if-env-changed=VOXVEIL_EDITION");
    for path in [
        "../native/windows/apo/out/Release/VoxveilApo.dll",
        "../native/windows/apo/out/Release/VoxveilApoCheck.exe",
        "../native/windows/apo/out/Release/VoxveilApoTarget.exe",
        "../native/windows/apo/VoxveilApo.inf",
        "../native/windows/apo/targets.ps1",
        "../native/windows/apo/extension.ps1",
        "../native/windows/apo/catalog.ps1",
        "../native/windows/apo/install.ps1",
        "../native/windows/apo/uninstall.ps1",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    stage_embedded_windows_audio_payload();
    tauri_build::build();
}
