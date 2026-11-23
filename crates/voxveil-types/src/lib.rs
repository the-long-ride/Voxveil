#![forbid(unsafe_code)]

pub mod ai;
pub mod audio;
pub mod processing;
pub mod routing;

pub use ai::{AiModel, AiPreferences, player_uses_ai};
pub use audio::{ChannelCount, SampleRate};
pub use processing::{
    AudioPlaybackSnapshot, AudioPlaybackStatus, AudioRouteChoice, ClassicSuppressionProfile, ProcessingBackendStatus, ProcessingEngineKind,
    ProcessingLoad, ProcessingMode, QualityPreference, UnitValue, VocalLevel,
    WindowsInterceptionPolicy,
};
pub use routing::{AudioBypassReason, AudioSourceCategory, OutputMode};
