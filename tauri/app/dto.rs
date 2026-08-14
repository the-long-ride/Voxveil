use serde::{Deserialize, Serialize};
use voxveil_types::{AudioBypassReason, AudioSourceCategory, OutputMode, ProcessingEngineKind, ProcessingLoad, ProcessingMode};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSourceDto {
    pub id: String,
    pub name: String,
    pub category: AudioSourceCategory,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bypass_reason: Option<AudioBypassReason>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppViewState {
    pub edition: String,
    pub master_enabled: bool,
    pub processing_mode: ProcessingMode,
    pub engine: ProcessingEngineKind,
    pub vocal_level: u8,
    pub quality: u8,
    pub output_mode: OutputMode,
    pub physical_output: String,
    pub virtual_output_available: bool,
    pub estimated_latency_ms: u16,
    pub load: ProcessingLoad,
    pub apps: Vec<AppSourceDto>,
}

impl Default for AppViewState {
    fn default() -> Self {
        Self {
            edition: crate::config::current_edition().as_str().into(),
            master_enabled: false,
            processing_mode: ProcessingMode::All,
            engine: ProcessingEngineKind::Auto,
            vocal_level: 100,
            quality: 50,
            output_mode: OutputMode::Physical,
            physical_output: "System Default".into(),
            virtual_output_available: false,
            estimated_latency_ms: 0,
            load: ProcessingLoad::Idle,
            apps: default_sources(),
        }
    }
}

fn default_sources() -> Vec<AppSourceDto> {
    vec![
        AppSourceDto { id: "media".into(), name: "Media".into(), category: AudioSourceCategory::Media, enabled: true, bypass_reason: None },
        AppSourceDto { id: "browser".into(), name: "Browser".into(), category: AudioSourceCategory::Media, enabled: true, bypass_reason: None },
        AppSourceDto { id: "game".into(), name: "Game".into(), category: AudioSourceCategory::Game, enabled: false, bypass_reason: None },
        AppSourceDto { id: "communication".into(), name: "Communication".into(), category: AudioSourceCategory::Communication, enabled: false, bypass_reason: Some(AudioBypassReason::Communication) },
    ]
}
