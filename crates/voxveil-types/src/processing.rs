use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioPlaybackStatus {
    #[default]
    Stopped,
    Loading,
    Playing,
    Paused,
    Ended,
    Faulted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioPlaybackSnapshot {
    pub status: AudioPlaybackStatus,
    pub file_name: Option<String>,
    pub output_endpoint_id: Option<String>,
    pub output_name: Option<String>,
    pub sample_rate: Option<u32>,
    pub position_frames: u64,
    pub total_frames: Option<u64>,
    pub error: Option<String>,
}

impl Default for AudioPlaybackSnapshot {
    fn default() -> Self {
        Self {
            status: AudioPlaybackStatus::Stopped,
            file_name: None,
            output_endpoint_id: None,
            output_name: None,
            sample_rate: None,
            position_frames: 0,
            total_frames: None,
            error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitValue(f32);

impl UnitValue {
    pub fn new(value: f32) -> Result<Self, &'static str> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err("value must be finite and between 0.0 and 1.0")
        }
    }

    pub const fn get(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VocalLevel(UnitValue);

impl VocalLevel {
    pub fn new(value: f32) -> Result<Self, &'static str> {
        UnitValue::new(value).map(Self)
    }

    pub const fn get(self) -> f32 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualityPreference(UnitValue);

impl QualityPreference {
    pub fn new(value: f32) -> Result<Self, &'static str> {
        UnitValue::new(value).map(Self)
    }

    pub const fn get(self) -> f32 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClassicSuppressionProfile {
    #[default]
    MusicPreservation,
    Balanced,
    Strong,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioRouteChoice {
    LegacyAutomatic,
    #[default]
    PhysicalApo,
    OwnedFilePlayback,
}

impl AudioRouteChoice {
    pub const fn interception_policy(self) -> WindowsInterceptionPolicy {
        match self {
            Self::LegacyAutomatic => WindowsInterceptionPolicy::Automatic,
            Self::PhysicalApo => WindowsInterceptionPolicy::PhysicalApoOnly,
            Self::OwnedFilePlayback => WindowsInterceptionPolicy::Disabled,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowsInterceptionPolicy {
    #[default]
    Automatic,
    PhysicalApoOnly,
    Disabled,
}

impl WindowsInterceptionPolicy {
    pub const fn allows_virtual_relay(self) -> bool {
        matches!(self, Self::Automatic)
    }

    pub const fn allows_physical_apo(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessingMode {
    All,
    PerApp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessingBackendStatus {
    Ready,
    ComponentRequired,
    RoutingRequired,
    Unsupported,
    Faulted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingLoad {
    Idle,
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingEngineKind {
    Auto,
    Dsp,
    Ai,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_values_reject_out_of_range_and_non_finite_values() {
        assert!(VocalLevel::new(-0.1).is_err());
        assert!(VocalLevel::new(1.1).is_err());
        assert!(VocalLevel::new(f32::NAN).is_err());
        assert_eq!(VocalLevel::new(0.0).unwrap().get(), 0.0);
        assert_eq!(VocalLevel::new(1.0).unwrap().get(), 1.0);
    }

    #[test]
    fn quality_uses_the_same_unit_interval() {
        assert!(QualityPreference::new(0.5).is_ok());
        assert!(QualityPreference::new(2.0).is_err());
    }

    #[test]
    fn classic_suppression_defaults_to_music_preservation() {
        assert_eq!(
            ClassicSuppressionProfile::default(),
            ClassicSuppressionProfile::MusicPreservation
        );
        assert_ne!(
            ClassicSuppressionProfile::MusicPreservation,
            ClassicSuppressionProfile::Balanced
        );
    }

    #[test]
    fn audio_route_choices_have_stable_kebab_case_names() {
        assert_eq!(
            serde_json::to_string(&AudioRouteChoice::LegacyAutomatic).unwrap(),
            "\"legacy-automatic\""
        );
        assert_eq!(
            serde_json::to_string(&AudioRouteChoice::PhysicalApo).unwrap(),
            "\"physical-apo\""
        );
        assert_eq!(
            serde_json::to_string(&AudioRouteChoice::OwnedFilePlayback).unwrap(),
            "\"owned-file-playback\""
        );
    }

    #[test]
    fn only_legacy_automatic_policy_allows_virtual_relays() {
        assert!(WindowsInterceptionPolicy::Automatic.allows_virtual_relay());
        assert!(!WindowsInterceptionPolicy::PhysicalApoOnly.allows_virtual_relay());
    }
}
