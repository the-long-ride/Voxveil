use std::sync::atomic::{AtomicU8, Ordering};

use voxveil_types::ClassicSuppressionProfile;

const MUSIC_PRESERVATION: u8 = 0;
const BALANCED: u8 = 1;

static CLASSIC_SUPPRESSION_PROFILE: AtomicU8 = AtomicU8::new(MUSIC_PRESERVATION);

pub(crate) fn set_classic_suppression_profile(profile: ClassicSuppressionProfile) {
    CLASSIC_SUPPRESSION_PROFILE.store(encode(profile), Ordering::Release);
}

pub(crate) fn classic_suppression_profile() -> ClassicSuppressionProfile {
    decode(CLASSIC_SUPPRESSION_PROFILE.load(Ordering::Acquire))
}

const fn encode(profile: ClassicSuppressionProfile) -> u8 {
    match profile {
        ClassicSuppressionProfile::MusicPreservation => MUSIC_PRESERVATION,
        ClassicSuppressionProfile::Balanced => BALANCED,
    }
}

const fn decode(value: u8) -> ClassicSuppressionProfile {
    match value {
        BALANCED => ClassicSuppressionProfile::Balanced,
        _ => ClassicSuppressionProfile::MusicPreservation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_wire_values_fail_safe_to_music_preservation() {
        assert_eq!(
            decode(255),
            ClassicSuppressionProfile::MusicPreservation
        );
        assert_eq!(decode(BALANCED), ClassicSuppressionProfile::Balanced);
    }
}
