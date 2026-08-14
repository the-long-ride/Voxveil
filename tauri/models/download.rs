use std::{
    fs,
    io::{Read, Write},
};

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use super::{catalog::ModelDescriptor, storage};

const DOWNLOAD_TIMEOUT_SECONDS: u64 = 1_800;
const BUFFER_BYTES: usize = 64 * 1024;
const PROGRESS_STEP_BYTES: u64 = 1024 * 1024;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress<'a> {
    model_id: &'a str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn open_https_download(url: &str) -> Result<minreq::ResponseLazy, String> {
    let mut current = url.to_string();
    for _ in 0..=5 {
        if !current.starts_with("https://") {
            return Err("AI model download redirected to a non-HTTPS source".to_string());
        }
        let response = minreq::get(&current)
            .with_timeout(DOWNLOAD_TIMEOUT_SECONDS)
            .with_follow_redirects(false)
            .send_lazy()
            .map_err(|_| "AI model download could not be started".to_string())?;
        if matches!(response.status_code, 301 | 302 | 303 | 307 | 308) {
            current = header_value(&response.headers, "location")
                .filter(|location| location.starts_with("https://"))
                .ok_or_else(|| "AI model download returned an unsafe redirect".to_string())?
                .to_string();
            continue;
        }
        return Ok(response);
    }
    Err("AI model download exceeded the redirect limit".to_string())
}

fn content_length(headers: &[(String, String)]) -> Option<u64> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse().ok())
}

fn cleanup(path: &std::path::Path) {
    let _ = fs::remove_file(path);
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

pub fn install(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<(), String> {
    if storage::is_installed(app, descriptor)? {
        return Ok(());
    }
    let paths = storage::paths(app, descriptor)?;
    fs::create_dir_all(&paths.directory)
        .map_err(|_| "AI model directory could not be created".to_string())?;
    cleanup(&paths.temporary);

    let mut response = open_https_download(&descriptor.download_url)?;
    if response.status_code != 200 || !response.url.starts_with("https://") {
        return Err("AI model source returned an invalid response".to_string());
    }
    let total_bytes = content_length(&response.headers);
    if total_bytes.is_some_and(|bytes| bytes > descriptor.max_bytes) {
        return Err("AI model download exceeds the approved size limit".to_string());
    }

    let mut file = fs::File::create(&paths.temporary)
        .map_err(|_| "AI model temporary file could not be created".to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; BUFFER_BYTES];
    let mut downloaded = 0_u64;
    let mut next_progress = 0_u64;

    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|_| "AI model download was interrupted".to_string())?;
        if read == 0 {
            break;
        }
        downloaded += read as u64;
        if downloaded > descriptor.max_bytes {
            cleanup(&paths.temporary);
            return Err("AI model download exceeded the approved size limit".to_string());
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|_| "AI model could not be written to disk".to_string())?;
        if downloaded >= next_progress {
            let _ = app.emit(
                "ai-model-download-progress",
                DownloadProgress {
                    model_id: &descriptor.id,
                    downloaded_bytes: downloaded,
                    total_bytes,
                },
            );
            next_progress = downloaded.saturating_add(PROGRESS_STEP_BYTES);
        }
    }
    file.flush()
        .map_err(|_| "AI model could not be flushed to disk".to_string())?;
    file.sync_all()
        .map_err(|_| "AI model could not be synchronized to disk".to_string())?;
    drop(file);

    let digest = hasher.finalize();
    let actual_hash = lower_hex(digest.as_ref());
    if actual_hash != descriptor.sha256 {
        cleanup(&paths.temporary);
        return Err("AI model integrity verification failed".to_string());
    }
    cleanup(&paths.model);
    fs::rename(&paths.temporary, &paths.model)
        .map_err(|_| "verified AI model could not be installed".to_string())?;
    if let Err(error) = storage::write_receipt(&paths, descriptor, downloaded) {
        cleanup(&paths.model);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_length_case_insensitively() {
        let headers = vec![("Content-Length".into(), "123".into())];
        assert_eq!(content_length(&headers), Some(123));
    }

    #[test]
    fn encodes_sha256_digest_as_lowercase_hex() {
        let digest = Sha256::digest(b"abc");
        assert_eq!(
            lower_hex(digest.as_ref()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
