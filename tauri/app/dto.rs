use serde::{Deserialize, Serialize};
use voxveil_types::{
    AudioBypassReason, AudioSourceCategory, ClassicSuppressionProfile, OutputMode,
    ProcessingBackendStatus, ProcessingEngineKind, ProcessingLoad, ProcessingMode,
};

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
pub struct AudioOutputDto {
    pub endpoint_id: String,
    pub display_name: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAudioEndpointDto {
    pub endpoint_id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_name: Option<String>,
    pub is_default: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResultDto {
    pub endpoint_id: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppViewState {
    pub edition: String,
    pub master_enabled: bool,
    pub backend_status: ProcessingBackendStatus,
    pub backend_kind: Option<String>,
    pub processing_mode: ProcessingMode,
    pub per_app_processing_available: bool,
    pub engine: ProcessingEngineKind,
    pub classic_suppression_profile: ClassicSuppressionProfile,
    pub vocal_level: u8,
    pub quality: u8,
    pub output_mode: OutputMode,
    pub physical_output: String,
    pub physical_output_endpoint_id: Option<String>,
    pub virtual_output_available: bool,
    pub estimated_latency_ms: u16,
    pub load: ProcessingLoad,
    pub apps: Vec<AppSourceDto>,
}

impl AppViewState {
    pub fn apply_backend(&mut self, snapshot: &crate::platform::BackendSnapshot) {
        self.backend_status = snapshot.status;
        self.backend_kind = snapshot.backend_kind.clone();
        self.physical_output_endpoint_id = snapshot.physical_output_endpoint_id.clone();
        self.physical_output = snapshot
            .physical_output
            .clone()
            .unwrap_or_else(|| "System Default".into());
        self.per_app_processing_available = snapshot.per_app_available;
        if snapshot.status != ProcessingBackendStatus::Ready {
            self.master_enabled = false;
        }
        if !snapshot.per_app_available {
            self.processing_mode = ProcessingMode::All;
        }
    }
}

impl Default for AppViewState {
    fn default() -> Self {
        Self {
            edition: crate::config::current_edition().as_str().into(),
            master_enabled: false,
            backend_status: crate::platform::processing_backend_status(),
            backend_kind: None,
            processing_mode: ProcessingMode::All,
            per_app_processing_available: false,
            engine: ProcessingEngineKind::Auto,
            classic_suppression_profile: ClassicSuppressionProfile::default(),
            vocal_level: 100,
            quality: 50,
            output_mode: OutputMode::Physical,
            physical_output: "System Default".into(),
            physical_output_endpoint_id: None,
            virtual_output_available: false,
            estimated_latency_ms: 0,
            load: ProcessingLoad::Idle,
            apps: default_sources(),
        }
    }
}

fn default_sources() -> Vec<AppSourceDto> {
    vec![
        AppSourceDto {
            id: "media".into(),
            name: "Media".into(),
            category: AudioSourceCategory::Media,
            enabled: true,
            bypass_reason: None,
        },
        AppSourceDto {
            id: "browser".into(),
            name: "Browser".into(),
            category: AudioSourceCategory::Media,
            enabled: true,
            bypass_reason: None,
        },
        AppSourceDto {
            id: "game".into(),
            name: "Game".into(),
            category: AudioSourceCategory::Game,
            enabled: false,
            bypass_reason: None,
        },
        AppSourceDto {
            id: "communication".into(),
            name: "Communication".into(),
            category: AudioSourceCategory::Communication,
            enabled: false,
            bypass_reason: Some(AudioBypassReason::Communication),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::BackendSnapshot;

    #[test]
    fn missing_physical_output_clears_stale_display_name_and_active_state() {
        let mut state = AppViewState::default();
        state.physical_output = "Removed Speakers".into();
        state.physical_output_endpoint_id = Some("removed-speakers".into());
        state.master_enabled = true;

        state.apply_backend(&BackendSnapshot {
            status: ProcessingBackendStatus::RoutingRequired,
            backend_kind: Some("vb-cable-relay".into()),
            physical_output: None,
            physical_output_endpoint_id: None,
            per_app_available: false,
        });

        assert_eq!(state.physical_output, "System Default");
        assert_eq!(state.physical_output_endpoint_id, None);
        assert!(!state.master_enabled);
    }
}
