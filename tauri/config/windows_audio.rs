use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const FILE_NAME: &str = "windows-audio.json";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsAudioPreferences {
    pub physical_output_endpoint_id: Option<String>,
    pub preferred_interception: Option<String>,
}

fn preferences_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(FILE_NAME))
        .map_err(|error| format!("failed to resolve Voxveil config directory: {error}"))
}

pub fn load(app: &AppHandle) -> Result<WindowsAudioPreferences, String> {
    let path = preferences_path(app)?;
    if !path.is_file() {
        return Ok(WindowsAudioPreferences::default());
    }
    let bytes = fs::read(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub fn save(app: &AppHandle, preferences: &WindowsAudioPreferences) -> Result<(), String> {
    let path = preferences_path(app)?;
    let directory = path
        .parent()
        .ok_or_else(|| "Windows audio preference path has no parent directory".to_string())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;

    let temporary = path.with_file_name(format!("{FILE_NAME}.tmp"));
    let json = serde_json::to_vec_pretty(preferences)
        .map_err(|error| format!("failed to serialize Windows audio preferences: {error}"))?;
    fs::write(&temporary, json)
        .map_err(|error| format!("failed to write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("failed to replace {}: {error}", path.display())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_audio_preferences_round_trip() {
        let prefs = WindowsAudioPreferences {
            physical_output_endpoint_id: Some("speakers-id".into()),
            preferred_interception: None,
        };
        let json = serde_json::to_string(&prefs).unwrap();
        assert_eq!(
            serde_json::from_str::<WindowsAudioPreferences>(&json).unwrap(),
            prefs
        );
    }

    #[test]
    fn preferences_do_not_define_audio_payload_fields() {
        let json = serde_json::to_value(WindowsAudioPreferences::default()).unwrap();
        let object = json.as_object().unwrap();
        assert_eq!(object.len(), 2);
        assert!(object.contains_key("physicalOutputEndpointId"));
        assert!(object.contains_key("preferredInterception"));
    }
}
