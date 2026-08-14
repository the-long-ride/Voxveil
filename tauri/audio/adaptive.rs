use voxveil_types::ProcessingEngineKind;

use super::{engine::RuntimeEngine, quality::{QualityTier, quality_tier}};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeConditions {
    pub ai_available: bool,
    pub gpu_available: bool,
    pub npu_available: bool,
    pub battery_saver: bool,
    pub thermal_pressure: bool,
    pub cpu_pressure: bool,
    pub underrun_risk: bool,
}

fn accelerator_available(conditions: RuntimeConditions) -> bool {
    conditions.gpu_available || conditions.npu_available
}

fn ai_for_tier(tier: QualityTier, conditions: RuntimeConditions) -> RuntimeEngine {
    if !conditions.ai_available {
        return RuntimeEngine::ClassicDsp;
    }
    if conditions.underrun_risk || conditions.thermal_pressure || conditions.cpu_pressure || conditions.battery_saver {
        return RuntimeEngine::FastAi;
    }
    match tier {
        QualityTier::UltraLowLatency | QualityTier::LowLatency => RuntimeEngine::FastAi,
        QualityTier::Balanced => RuntimeEngine::BalancedAi,
        QualityTier::HighQuality if accelerator_available(conditions) => RuntimeEngine::HighQualityAi,
        QualityTier::HighQuality => RuntimeEngine::BalancedAi,
    }
}

pub fn select_runtime_engine(
    selected: ProcessingEngineKind,
    quality_percent: u8,
    conditions: RuntimeConditions,
) -> RuntimeEngine {
    match selected {
        ProcessingEngineKind::Dsp => RuntimeEngine::ClassicDsp,
        ProcessingEngineKind::Ai => ai_for_tier(quality_tier(quality_percent), conditions),
        ProcessingEngineKind::Auto => {
            if !conditions.ai_available || quality_percent <= 20 {
                RuntimeEngine::ClassicDsp
            } else {
                ai_for_tier(quality_tier(quality_percent), conditions)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn available() -> RuntimeConditions {
        RuntimeConditions { ai_available: true, gpu_available: true, ..Default::default() }
    }

    #[test]
    fn explicit_dsp_never_selects_ai() {
        assert_eq!(select_runtime_engine(ProcessingEngineKind::Dsp, 100, available()), RuntimeEngine::ClassicDsp);
    }

    #[test]
    fn auto_uses_dsp_when_ai_is_missing_or_latency_is_priority() {
        assert_eq!(select_runtime_engine(ProcessingEngineKind::Auto, 100, RuntimeConditions::default()), RuntimeEngine::ClassicDsp);
        assert_eq!(select_runtime_engine(ProcessingEngineKind::Auto, 10, available()), RuntimeEngine::ClassicDsp);
    }

    #[test]
    fn high_quality_requires_ai_and_an_accelerator() {
        assert_eq!(select_runtime_engine(ProcessingEngineKind::Auto, 90, available()), RuntimeEngine::HighQualityAi);
        let cpu_only = RuntimeConditions { ai_available: true, ..Default::default() };
        assert_eq!(select_runtime_engine(ProcessingEngineKind::Auto, 90, cpu_only), RuntimeEngine::BalancedAi);
    }

    #[test]
    fn pressure_degrades_but_never_exceeds_user_quality() {
        let pressured = RuntimeConditions { thermal_pressure: true, ..available() };
        assert_eq!(select_runtime_engine(ProcessingEngineKind::Auto, 90, pressured), RuntimeEngine::FastAi);
        assert_eq!(select_runtime_engine(ProcessingEngineKind::Auto, 30, available()), RuntimeEngine::FastAi);
    }
}
