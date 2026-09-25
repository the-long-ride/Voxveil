use super::*;

impl ProcessingController {
    pub fn open_owned_playback(
        &self,
        path: std::path::PathBuf,
        endpoint_id: Option<String>,
        vocal_level: u8,
        profile: ClassicSuppressionProfile,
    ) -> Result<AudioPlaybackSnapshot, String> {
        let endpoint = {
            #[cfg(target_os = "windows")]
            {
                let backend = self
                    .backend
                    .lock()
                    .map_err(|_| "Windows audio backend lock is poisoned".to_string())?;
                let outputs = backend.physical_outputs()?;
                match endpoint_id {
                    Some(endpoint_id) => outputs
                        .into_iter()
                        .find(|output| output.id == endpoint_id)
                        .ok_or_else(|| {
                            "The selected physical playback endpoint is no longer available"
                                .to_string()
                        })?,
                    None => outputs
                        .into_iter()
                        .find(|output| output.is_default)
                        .ok_or_else(|| "No physical playback endpoint is available".to_string())?,
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = endpoint_id;
                return Err("owned file playback is unavailable on this platform".into());
            }
        };
        self.playback
            .lock()
            .map_err(|_| "local playback lock is poisoned".to_string())?
            .open(path, endpoint, vocal_level, profile)
    }

    pub fn playback_snapshot(&self) -> AudioPlaybackSnapshot {
        self.playback
            .lock()
            .map(|playback| playback.snapshot())
            .unwrap_or_else(|_| AudioPlaybackSnapshot {
                status: AudioPlaybackStatus::Faulted,
                error: Some("local playback lock is poisoned".into()),
                ..AudioPlaybackSnapshot::default()
            })
    }

    pub fn pause_owned_playback(&self) -> Result<(), String> {
        self.playback
            .lock()
            .map_err(|_| "local playback lock is poisoned".to_string())?
            .pause()
    }

    pub fn resume_owned_playback(&self) -> Result<(), String> {
        self.playback
            .lock()
            .map_err(|_| "local playback lock is poisoned".to_string())?
            .resume()
    }

    pub fn seek_owned_playback(&self, frame: u64) -> Result<(), String> {
        self.playback
            .lock()
            .map_err(|_| "local playback lock is poisoned".to_string())?
            .seek(frame)
    }

    pub fn stop_owned_playback(&self) -> Result<(), String> {
        self.playback
            .lock()
            .map_err(|_| "local playback lock is poisoned".to_string())?
            .stop()
    }
}
