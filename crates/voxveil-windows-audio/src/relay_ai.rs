use voxveil_ai::player::AiPlaybackConfig;
use voxveil_types::{ProcessingEngineKind, SystemAiStatus};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayAiOptions {
    pub config: Option<AiPlaybackConfig>,
    pub status: SystemAiStatus,
    pub engine: ProcessingEngineKind,
    pub log_path: Option<std::path::PathBuf>,
}
impl Default for RelayAiOptions {
    fn default() -> Self {
        Self {
            config: None,
            status: SystemAiStatus::Off,
            engine: ProcessingEngineKind::Auto,
            log_path: None,
        }
    }
}
impl RelayAiOptions {
    pub fn update(&mut self, next: Self, running: bool) -> bool {
        let restart = running && *self != next;
        *self = next;
        restart
    }
}

#[cfg(test)]
#[path = "relay_ai_tests.rs"]
mod tests;
