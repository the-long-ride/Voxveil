use std::{
    fs::{File, OpenOptions},
    sync::atomic::{AtomicU64, Ordering},
};

pub(super) fn anonymous_file() -> Result<File, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..100 {
        let path = std::env::temp_dir().join(format!(
            "voxveil-stems-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            const GENERIC_READ_WRITE_DELETE: u32 = 0xC0010000;
            const FILE_FLAG_DELETE_ON_CLOSE: u32 = 0x04000000;
            options
                .access_mode(GENERIC_READ_WRITE_DELETE)
                .custom_flags(FILE_FLAG_DELETE_ON_CLOSE)
                .share_mode(0);
        }
        match options.open(&path) {
            Ok(file) => {
                #[cfg(unix)]
                std::fs::remove_file(&path)
                    .map_err(|_| "AI temporary stem cache could not be unlinked")?;
                return Ok(file);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err("AI temporary stem cache could not be created".into()),
        }
    }
    Err("AI temporary stem cache could not get a unique name".into())
}
