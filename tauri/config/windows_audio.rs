use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};
use tauri::{AppHandle, Manager};
use voxveil_types::{AudioRouteChoice, ClassicSuppressionProfile};

const FILE_NAME: &str = "windows-audio.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LegacyInterceptionPreference {
    Apo,
    VbCableRelay,
    VoxveilCableRelay,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsAudioPreferences {
    pub physical_output_endpoint_id: Option<String>,
    pub preferred_interception: Option<LegacyInterceptionPreference>,
    pub route_choice: AudioRouteChoice,
    pub classic_suppression_profile: ClassicSuppressionProfile,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedWindowsAudioPreferences {
    #[serde(default)]
    physical_output_endpoint_id: Option<String>,
    #[serde(default)]
    preferred_interception: Option<String>,
    #[serde(default)]
    route_choice: Option<String>,
    #[serde(default)]
    classic_suppression_profile: ClassicSuppressionProfile,
}

impl<'de> Deserialize<'de> for WindowsAudioPreferences {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let persisted = PersistedWindowsAudioPreferences::deserialize(deserializer)?;
        let route_choice = persisted
            .route_choice
            .as_deref()
            .map(parse_route_choice)
            .transpose()
            .map_err(serde::de::Error::custom)?
            .unwrap_or(AudioRouteChoice::LegacyAutomatic);
        let preferred_interception = persisted
            .preferred_interception
            .as_deref()
            .map(parse_legacy_interception)
            .transpose()
            .map_err(serde::de::Error::custom)?;

        Ok(Self {
            physical_output_endpoint_id: persisted.physical_output_endpoint_id,
            preferred_interception,
            route_choice,
            classic_suppression_profile: persisted.classic_suppression_profile,
        })
    }
}

fn parse_route_choice(value: &str) -> Result<AudioRouteChoice, String> {
    match value {
        "legacy-automatic" => Ok(AudioRouteChoice::LegacyAutomatic),
        "physical-apo" => Ok(AudioRouteChoice::PhysicalApo),
        "owned-file-playback" => Ok(AudioRouteChoice::OwnedFilePlayback),
        _ => Err(format!("unknown Windows audio route choice `{value}`")),
    }
}

fn parse_legacy_interception(value: &str) -> Result<LegacyInterceptionPreference, String> {
    match value {
        "apo" => Ok(LegacyInterceptionPreference::Apo),
        "vb-cable-relay" => Ok(LegacyInterceptionPreference::VbCableRelay),
        "voxveil-cable-relay" => Ok(LegacyInterceptionPreference::VoxveilCableRelay),
        _ => Err(format!(
            "unknown legacy Windows audio preference `preferredInterception`: `{value}`"
        )),
    }
}

fn decode_preferences(bytes: &[u8]) -> Result<WindowsAudioPreferences, String> {
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
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

    let persisted_json = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or_default();
    if !persisted_json.is_object() {
        return Ok(WindowsAudioPreferences::default());
    }

    decode_preferences(&bytes)
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
            route_choice: AudioRouteChoice::PhysicalApo,
            classic_suppression_profile: ClassicSuppressionProfile::Balanced,
        };
        let json = serde_json::to_string(&prefs).unwrap();
        assert_eq!(
            serde_json::from_str::<WindowsAudioPreferences>(&json).unwrap(),
            prefs
        );
    }

    #[test]
    fn legacy_preferences_default_to_music_preservation() {
        let prefs = serde_json::from_str::<WindowsAudioPreferences>(
            r#"{"physicalOutputEndpointId":"speakers-id","preferredInterception":null}"#,
        )
        .unwrap();
        assert_eq!(
            prefs.classic_suppression_profile,
            ClassicSuppressionProfile::MusicPreservation
        );
    }

    #[test]
    fn preferences_do_not_define_audio_payload_fields() {
        let json = serde_json::to_value(WindowsAudioPreferences::default()).unwrap();
        let object = json.as_object().unwrap();
        assert_eq!(object.len(), 4);
        assert!(object.contains_key("physicalOutputEndpointId"));
        assert!(object.contains_key("preferredInterception"));
        assert_eq!(
            object.get("routeChoice").and_then(|value| value.as_str()),
            Some("physical-apo")
        );
        assert_eq!(
            object.get("classicSuppressionProfile").and_then(|value| value.as_str()),
            Some("music-preservation")
        );
    }

    #[test]
    fn new_install_defaults_to_physical_apo() {
        assert_eq!(
            WindowsAudioPreferences::default().route_choice,
            AudioRouteChoice::PhysicalApo
        );
    }

    #[test]
    fn old_preferences_without_route_migrate_to_legacy_automatic() {
        let prefs = decode_preferences(
            br#"{"physicalOutputEndpointId":"speakers-id","preferredInterception":"vb-cable-relay"}"#,
        )
        .unwrap();

        assert_eq!(prefs.route_choice, AudioRouteChoice::LegacyAutomatic);
        assert_eq!(
            prefs.preferred_interception,
            Some(LegacyInterceptionPreference::VbCableRelay)
        );
    }

    #[test]
    fn unknown_legacy_interception_is_rejected_instead_of_silently_migrated() {
        let result = decode_preferences(br#"{"preferredInterception":"mystery-route"}"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("preferredInterception"));
    }

    #[test]
    fn unknown_explicit_route_is_rejected() {
        let result = decode_preferences(br#"{"routeChoice":"virtual-fallback"}"#);
        assert!(result.unwrap_err().contains("unknown Windows audio route choice"));
    }
}
