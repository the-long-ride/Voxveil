use voxveil_types::ProcessingEngineKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEngine {
    HighQualityAi,
    BalancedAi,
    FastAi,
    ClassicDsp,
    Bypass,
}

pub fn fallback_chain(selected: ProcessingEngineKind) -> &'static [RuntimeEngine] {
    match selected {
        ProcessingEngineKind::Dsp => &[RuntimeEngine::ClassicDsp, RuntimeEngine::Bypass],
        ProcessingEngineKind::Ai => &[
            RuntimeEngine::HighQualityAi,
            RuntimeEngine::BalancedAi,
            RuntimeEngine::FastAi,
            RuntimeEngine::ClassicDsp,
            RuntimeEngine::Bypass,
        ],
        ProcessingEngineKind::Auto => &[
            RuntimeEngine::BalancedAi,
            RuntimeEngine::FastAi,
            RuntimeEngine::ClassicDsp,
            RuntimeEngine::Bypass,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_dsp_never_silently_switches_to_ai() {
        let chain = fallback_chain(ProcessingEngineKind::Dsp);
        assert_eq!(chain, &[RuntimeEngine::ClassicDsp, RuntimeEngine::Bypass]);
    }

    #[test]
    fn auto_degrades_toward_continuity() {
        let chain = fallback_chain(ProcessingEngineKind::Auto);
        assert_eq!(chain.last(), Some(&RuntimeEngine::Bypass));
        assert!(chain.contains(&RuntimeEngine::ClassicDsp));
    }
}
