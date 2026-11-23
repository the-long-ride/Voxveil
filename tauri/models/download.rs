use super::{catalog::ModelDescriptor, storage, storage_files, transfer};
use std::fs;
use tauri::{AppHandle, Emitter};

const DOWNLOAD_TIMEOUT_SECONDS: u64 = 1_800;
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
    header_value(headers, "content-length").and_then(|value| value.parse().ok())
}

fn cleanup(path: &std::path::Path) {
    let _ = fs::remove_file(path);
}

pub fn install(app: &AppHandle, descriptor: &ModelDescriptor) -> Result<(), String> {
    if storage::is_installed(app, descriptor)? {
        return Ok(());
    }
    let paths = storage::paths(app, descriptor)?;
    fs::create_dir_all(&paths.directory)
        .map_err(|_| "AI model directory could not be created".to_string())?;
    cleanup(&paths.receipt);
    let result = install_files(app, descriptor, &paths);
    if result.is_err() {
        cleanup(&paths.receipt);
        for file in &paths.files {
            cleanup(&file.temporary);
            cleanup(&file.model);
        }
    }
    result
}

fn install_files(
    app: &AppHandle,
    descriptor: &ModelDescriptor,
    paths: &storage_files::ModelPaths,
) -> Result<(), String> {
    let mut bytes = Vec::new();
    let mut offset = 0;
    let mut next_progress = 0;
    for (file, path) in descriptor.files.iter().zip(&paths.files) {
        cleanup(&path.temporary);
        let mut response = open_https_download(&file.download_url)?;
        if response.status_code != 200 || !response.url.starts_with("https://") {
            return Err("AI model source returned an invalid response".to_string());
        }
        if content_length(&response.headers).is_some_and(|bytes| bytes > file.max_bytes) {
            return Err("AI model download exceeds the approved size limit".to_string());
        }
        let mut output = fs::File::create(&path.temporary)
            .map_err(|_| "AI model temporary file could not be created".to_string())?;
        let downloaded = transfer::copy_verified(&mut response, &mut output, file, |downloaded| {
            let aggregate = offset + downloaded;
            if aggregate >= next_progress {
                let _ = app.emit(
                    "ai-model-download-progress",
                    DownloadProgress {
                        model_id: &descriptor.id,
                        downloaded_bytes: aggregate,
                        total_bytes: None,
                    },
                );
                next_progress = aggregate.saturating_add(PROGRESS_STEP_BYTES);
            }
        })?;
        output
            .sync_all()
            .map_err(|_| "AI model could not be synchronized to disk".to_string())?;
        bytes.push(downloaded);
        offset += downloaded;
    }
    for path in &paths.files {
        cleanup(&path.model);
        fs::rename(&path.temporary, &path.model)
            .map_err(|_| "verified AI model could not be installed".to_string())?;
    }
    storage_files::write_receipt(paths, descriptor, &bytes)?;
    let _ = app.emit(
        "ai-model-download-progress",
        DownloadProgress {
            model_id: &descriptor.id,
            downloaded_bytes: offset,
            total_bytes: Some(offset),
        },
    );
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
}
