use crate::ProcessingEngineKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiModel {
    #[default]
    #[serde(rename = "spleeter-2stems-fp16")]
    Spleeter,
    #[serde(rename = "mdx-inst-hq3")]
    MdxInstHq3,
}

impl AiModel {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Spleeter => "spleeter-2stems-fp16",
            Self::MdxInstHq3 => "mdx-inst-hq3",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AiPreferences {
    pub model: AiModel,
    pub system_wide: bool,
}

impl AiPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.system_wide && self.model != AiModel::Spleeter {
            Err("System-wide AI supports Spleeter only; MDX cannot sustain real time".into())
        } else {
            Ok(())
        }
    }
}

pub fn player_uses_ai(engine: ProcessingEngineKind, ready: bool) -> Result<bool, String> {
    match engine {
        ProcessingEngineKind::Dsp => Ok(false),
        ProcessingEngineKind::Auto => Ok(ready),
        ProcessingEngineKind::Ai if ready => Ok(true),
        ProcessingEngineKind::Ai => Err(
            "AI is unavailable: install the selected model and the app-owned ONNX Runtime".into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn engine_selection_never_silently_falls_back_from_explicit_ai() {
        for ready in [false, true] {
            assert_eq!(player_uses_ai(ProcessingEngineKind::Auto, ready), Ok(ready));
            assert_eq!(player_uses_ai(ProcessingEngineKind::Dsp, ready), Ok(false));
        }
        assert!(player_uses_ai(ProcessingEngineKind::Ai, false).is_err());
        assert_eq!(player_uses_ai(ProcessingEngineKind::Ai, true), Ok(true));
    }
    #[test]
    fn defaults_and_old_settings_do_not_opt_into_delayed_audio() {
        let prefs: AiPreferences = serde_json::from_str("{}").unwrap();
        assert_eq!(prefs, AiPreferences::default());
        assert_eq!(prefs.model, AiModel::Spleeter);
        assert!(!prefs.system_wide);
        assert!(
            AiPreferences {
                model: AiModel::MdxInstHq3,
                system_wide: true
            }
            .validate()
            .is_err()
        );
    }
}
