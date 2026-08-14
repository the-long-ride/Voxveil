#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityTier {
    UltraLowLatency,
    LowLatency,
    Balanced,
    HighQuality,
}

pub fn quality_tier(percent: u8) -> QualityTier {
    match percent {
        0..=20 => QualityTier::UltraLowLatency,
        21..=45 => QualityTier::LowLatency,
        46..=70 => QualityTier::Balanced,
        _ => QualityTier::HighQuality,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_quality_to_approved_tiers() {
        assert_eq!(quality_tier(0), QualityTier::UltraLowLatency);
        assert_eq!(quality_tier(21), QualityTier::LowLatency);
        assert_eq!(quality_tier(56), QualityTier::Balanced);
        assert_eq!(quality_tier(100), QualityTier::HighQuality);
    }
}
