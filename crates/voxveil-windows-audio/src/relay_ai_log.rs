use std::{
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn fallback(path: Option<&Path>, reason: &str) {
    eprintln!("Voxveil relay: {reason}; falling back to DSP");
    let Some(path) = path else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let rotate = fs::metadata(path).is_ok_and(|metadata| metadata.len() >= 1024 * 1024);
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(!rotate)
        .truncate(rotate)
        .open(path)
    {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(file, "{timestamp} {reason}; falling back to DSP");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logs_fallbacks_rotates_bounded_output_and_tolerates_unwritable_paths() {
        let root = std::env::temp_dir().join(format!("voxveil-relay-log-{}", std::process::id()));
        let path = root.join("logs/relay-ai.log");
        fallback(None, "no configured log");
        fallback(Some(&path), "deadline missed");
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("deadline missed; falling back to DSP")
        );
        fs::write(&path, vec![0; 1024 * 1024]).unwrap();
        fallback(Some(&path), "runtime failed");
        assert!(fs::metadata(&path).unwrap().len() < 1024);
        fallback(Some(&path.join("invalid")), "cannot log here");
        fs::remove_dir_all(root).unwrap();
    }
}
