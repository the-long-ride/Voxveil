#![forbid(unsafe_code)]

pub mod ai;
pub mod audio;
pub mod processing;
pub mod player_engine;
pub mod routing;

pub use ai::{AiModel, AiPreferences, player_uses_ai};
pub use ai::{SystemAiStatus, relay_ai_status};
pub use audio::{ChannelCount, SampleRate};
pub use processing::{
    AudioPlaybackSnapshot, AudioPlaybackStatus, AudioRouteChoice, ClassicSuppressionProfile, ProcessingBackendStatus, ProcessingEngineKind,
    ProcessingLoad, ProcessingMode, QualityPreference, UnitValue, VocalLevel,
    WindowsInterceptionPolicy,
};
pub use routing::{AudioBypassReason, AudioSourceCategory, OutputMode};
