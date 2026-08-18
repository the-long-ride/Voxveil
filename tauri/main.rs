fn main() {
    if std::env::args().any(|arg| arg == "--verify-embedded-system-audio") {
        match voxveil_app::verify_embedded_system_audio_payload() {
            Ok(()) => {
                println!("embedded Windows system-audio payload is valid");
                return;
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
    voxveil_app::run();
}
