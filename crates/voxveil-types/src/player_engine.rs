use crate::{ProcessingEngineKind, player_uses_ai};

pub fn player_uses_ai_at_rate(engine: ProcessingEngineKind, ready: bool, rate: u32) -> Result<bool, String> {
    let supported = matches!(rate, 44100 | 48000);
    if engine == ProcessingEngineKind::Ai && !supported {
        return Err(format!("AI playback supports 44.1 or 48 kHz input, not {rate} Hz; select Auto or DSP for this file"));
    }
    player_uses_ai(engine, ready && supported)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auto_uses_dsp_for_unsupported_rates_and_explicit_ai_explains_why() {
        for rate in [8000, 22050, 96000, 192000] {
            assert_eq!(player_uses_ai_at_rate(ProcessingEngineKind::Auto, true, rate), Ok(false));
            assert!(player_uses_ai_at_rate(ProcessingEngineKind::Ai, true, rate).unwrap_err().contains(&format!("{rate} Hz")));
            assert_eq!(player_uses_ai_at_rate(ProcessingEngineKind::Dsp, true, rate), Ok(false));
        }
        for rate in [44100, 48000] {
            assert_eq!(player_uses_ai_at_rate(ProcessingEngineKind::Auto, true, rate), Ok(true));
            assert_eq!(player_uses_ai_at_rate(ProcessingEngineKind::Auto, false, rate), Ok(false));
        }
    }
}
