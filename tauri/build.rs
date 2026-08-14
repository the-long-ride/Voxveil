fn main() {
    println!("cargo:rerun-if-env-changed=VOXVEIL_EDITION");
    tauri_build::build();
}
