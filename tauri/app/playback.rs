use tauri::State;
use voxveil_types::{AudioPlaybackSnapshot, AudioRouteChoice};

use super::state::AppState;
use crate::platform::ProcessingController;

#[cfg(any(target_os = "windows", test))]
fn prepare_with_selection<T>(
    state: &AppState,
    prepare: impl FnOnce(
        voxveil_types::ProcessingEngineKind,
        voxveil_types::AiModel,
    ) -> Result<T, String>,
) -> Result<T, String> {
    let (engine, model) = {
        let current = state.lock()?;
        (current.engine, current.ai.model)
    };
    prepare(engine, model)
}

#[tauri::command]
pub async fn open_audio_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
) -> Result<AudioPlaybackSnapshot, String> {
    #[cfg(target_os = "windows")]
    {
        let (route, endpoint, level, profile) = {
            let current = state.lock()?;
            (
                current.audio_route_choice,
                current.physical_output_endpoint_id.clone(),
                current.vocal_level,
                current.classic_suppression_profile,
            )
        };
        if route != AudioRouteChoice::OwnedFilePlayback {
            return Err("select the local-file playback route before opening a file".into());
        }

        let Some(path) = rfd::FileDialog::new()
            .set_title("Open a music file in Voxveil")
            .add_filter("Music files", &["mp3", "wav", "flac", "ogg"])
            .pick_file()
        else {
            return Ok(controller.playback_snapshot());
        };

        controller.set_enabled(false, level)?;
        let ai = prepare_with_selection(&state, |engine, model| {
            let ready = engine != voxveil_types::ProcessingEngineKind::Dsp
                && crate::models::ai_runtime_ready(&app).unwrap_or(false);
            let rate = voxveil_media::MediaDecoder::open(&path)
                .map_err(|e| e.to_string())?
                .info()
                .sample_rate;
            let use_ai = voxveil_types::player_engine::player_uses_ai_at_rate(engine, ready, rate)?;
            if use_ai {
                Ok(Some(crate::models::player_config(&app, model)?))
            } else {
                Ok(None)
            }
        })?;
        controller.open_owned_playback(path, endpoint, level, profile, ai)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state, controller);
        Err("owned file playback is available only in Windows builds".into())
    }
}

#[tauri::command]
pub async fn get_playback_state(
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
) -> Result<AudioPlaybackSnapshot, String> {
    let route = state.lock()?.audio_route_choice;
    if route != AudioRouteChoice::OwnedFilePlayback {
        return Ok(AudioPlaybackSnapshot::default());
    }
    Ok(controller.playback_snapshot())
}

#[tauri::command]
pub async fn pause_playback(controller: State<'_, ProcessingController>) -> Result<(), String> {
    controller.pause_owned_playback()
}

#[tauri::command]
pub async fn resume_playback(controller: State<'_, ProcessingController>) -> Result<(), String> {
    controller.resume_owned_playback()
}

#[tauri::command]
pub async fn seek_playback(
    controller: State<'_, ProcessingController>,
    position_frames: u64,
) -> Result<(), String> {
    controller.seek_owned_playback(position_frames)
}

#[tauri::command]
pub async fn stop_playback(controller: State<'_, ProcessingController>) -> Result<(), String> {
    controller.stop_owned_playback()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::dto::AppViewState;
    use std::{
        sync::{Arc, mpsc},
        time::Duration,
    };

    #[test]
    fn slow_file_preparation_does_not_block_playback_state_reads() {
        let state = Arc::new(AppState::default());
        let preparing = Arc::clone(&state);
        let (entered, scan_started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let scan = std::thread::spawn(move || {
            prepare_with_selection(&preparing, |engine, model| {
                assert_eq!(engine, voxveil_types::ProcessingEngineKind::Auto);
                assert_eq!(model, voxveil_types::AiModel::Spleeter);
                entered.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(2)).unwrap();
                Err::<(), _>("probe failed".to_string())
            })
        });
        scan_started.recv_timeout(Duration::from_secs(2)).unwrap();
        let (read, available) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let _current = state.lock().unwrap();
            read.send(()).unwrap();
        });
        let responsive = available.recv_timeout(Duration::from_millis(500)).is_ok();
        release.send(()).unwrap();
        assert_eq!(scan.join().unwrap().unwrap_err(), "probe failed");
        reader.join().unwrap();
        assert!(responsive, "state reads waited for the file scan");
    }

    #[test]
    fn selected_file_path_is_not_part_of_the_frontend_app_state() {
        let serialized = serde_json::to_value(AppViewState::default()).unwrap();
        assert!(serialized.get("playback").is_some());
        assert!(serialized.get("filePath").is_none());
        assert!(serialized.get("path").is_none());
    }
}
