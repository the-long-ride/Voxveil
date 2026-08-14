use voxveil_types::{AudioSourceCategory, ProcessingMode};

use crate::{AppOverride, GlobalRoutingSettings, ResolvedProcessing, SourceInfo};

pub fn resolve_processing(
    global: GlobalRoutingSettings,
    source: &SourceInfo,
    app_override: Option<AppOverride>,
) -> ResolvedProcessing {
    let communication = source.category == AudioSourceCategory::Communication;
    let override_value = app_override.unwrap_or_default();
    let mode_enabled = match global.mode {
        ProcessingMode::All => global.enabled,
        ProcessingMode::PerApp => global.enabled && override_value.enabled.unwrap_or(false),
    };

    ResolvedProcessing {
        enabled: mode_enabled && !communication && override_value.enabled.unwrap_or(true),
        bypass_communication: communication,
        vocal_level: override_value.vocal_level.unwrap_or(global.vocal_level),
        quality: override_value.quality.unwrap_or(global.quality),
        engine: override_value.engine.unwrap_or(global.engine),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxveil_types::{ProcessingEngineKind, QualityPreference, VocalLevel};

    fn global(mode: ProcessingMode) -> GlobalRoutingSettings {
        GlobalRoutingSettings {
            enabled: true,
            mode,
            vocal_level: VocalLevel::new(0.2).unwrap(),
            quality: QualityPreference::new(0.6).unwrap(),
            engine: ProcessingEngineKind::Auto,
        }
    }

    fn source(category: AudioSourceCategory) -> SourceInfo {
        SourceInfo { id: "app".into(), name: "App".into(), category }
    }

    #[test]
    fn all_output_processes_media_by_default() {
        let result = resolve_processing(global(ProcessingMode::All), &source(AudioSourceCategory::Media), None);
        assert!(result.enabled);
    }

    #[test]
    fn per_app_requires_explicit_enable() {
        let result = resolve_processing(global(ProcessingMode::PerApp), &source(AudioSourceCategory::Media), None);
        assert!(!result.enabled);
        let enabled = AppOverride { enabled: Some(true), ..Default::default() };
        assert!(resolve_processing(global(ProcessingMode::PerApp), &source(AudioSourceCategory::Media), Some(enabled)).enabled);
    }

    #[test]
    fn communication_audio_is_bypassed_even_in_all_output_mode() {
        let result = resolve_processing(global(ProcessingMode::All), &source(AudioSourceCategory::Communication), None);
        assert!(!result.enabled);
        assert!(result.bypass_communication);
    }

    #[test]
    fn partial_override_inherits_other_global_values() {
        let quality = QualityPreference::new(0.9).unwrap();
        let override_value = AppOverride { quality: Some(quality), ..Default::default() };
        let result = resolve_processing(global(ProcessingMode::All), &source(AudioSourceCategory::Media), Some(override_value));
        assert_eq!(result.quality, quality);
        assert_eq!(result.vocal_level, VocalLevel::new(0.2).unwrap());
    }
}
