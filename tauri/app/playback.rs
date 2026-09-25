use tauri::State;
use voxveil_types::{AudioPlaybackSnapshot, AudioRouteChoice};

use super::state::AppState;
use crate::platform::ProcessingController;

#[tauri::command]
pub async fn open_audio_file(
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
        controller.open_owned_playback(path, endpoint, level, profile)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (state, controller);
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
    use crate::app::dto::AppViewState;

    #[test]
    fn selected_file_path_is_not_part_of_the_frontend_app_state() {
        let serialized = serde_json::to_value(AppViewState::default()).unwrap();
        assert!(serialized.get("playback").is_some());
        assert!(serialized.get("filePath").is_none());
        assert!(serialized.get("path").is_none());
    }
}
