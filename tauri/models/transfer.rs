use super::catalog::ModelFile;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

pub fn copy_verified(
    reader: &mut impl Read,
    writer: &mut impl Write,
    file: &ModelFile,
    mut progress: impl FnMut(u64),
) -> Result<u64, String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut downloaded = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| "AI model download was interrupted".to_string())?;
        if read == 0 {
            break;
        }
        downloaded += read as u64;
        if downloaded > file.max_bytes {
            return Err("AI model download exceeded the approved size limit".to_string());
        }
        hasher.update(&buffer[..read]);
        writer
            .write_all(&buffer[..read])
            .map_err(|_| "AI model could not be written to disk".to_string())?;
        progress(downloaded);
    }
    if downloaded == 0 || hex(hasher.finalize().as_slice()) != file.sha256 {
        return Err("AI model integrity verification failed".to_string());
    }
    writer
        .flush()
        .map_err(|_| "AI model could not be flushed to disk".to_string())?;
    Ok(downloaded)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
