use voxveil_types::{
    AudioSourceCategory, ProcessingEngineKind, ProcessingMode, QualityPreference, VocalLevel,
};

#[derive(Clone, Debug)]
pub struct SourceInfo {
    pub id: String,
    pub name: String,
    pub category: AudioSourceCategory,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AppOverride {
    pub enabled: Option<bool>,
    pub vocal_level: Option<VocalLevel>,
    pub quality: Option<QualityPreference>,
    pub engine: Option<ProcessingEngineKind>,
}

#[derive(Clone, Copy, Debug)]
pub struct GlobalRoutingSettings {
    pub enabled: bool,
    pub mode: ProcessingMode,
    pub vocal_level: VocalLevel,
    pub quality: QualityPreference,
    pub engine: ProcessingEngineKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedProcessing {
    pub enabled: bool,
    pub bypass_communication: bool,
    pub vocal_level: VocalLevel,
    pub quality: QualityPreference,
    pub engine: ProcessingEngineKind,
}
