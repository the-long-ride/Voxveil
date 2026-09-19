#pragma once

#include <audioenginebaseapo.h>
#include <audioengineextensionapo.h>

namespace voxveil {

enum class ApoInitFlavor {
    Invalid,
    V1,
    V2,
    V3,
};

enum class ClassicSuppressionProfile : LONG {
    MusicPreservation = 0,
    Balanced = 1,
};

constexpr ApoInitFlavor ClassifyInitSize(UINT32 bytes) noexcept {
    if (bytes == sizeof(APOInitSystemEffects3)) return ApoInitFlavor::V3;
    if (bytes == sizeof(APOInitSystemEffects2)) return ApoInitFlavor::V2;
    if (bytes == sizeof(APOInitSystemEffects)) return ApoInitFlavor::V1;
    return ApoInitFlavor::Invalid;
}

constexpr bool ShouldCountLoadedInstance(ApoInitFlavor flavor, bool discoveryOnly) noexcept {
    return flavor != ApoInitFlavor::Invalid && !(flavor == ApoInitFlavor::V3 && discoveryOnly);
}

constexpr bool ShouldProcess(bool appEnabled, bool systemEffectEnabled, LONG vocalPercent) noexcept {
    return appEnabled && systemEffectEnabled && vocalPercent < 100;
}

constexpr ClassicSuppressionProfile NormalizeProfile(LONG value) noexcept {
    return value == static_cast<LONG>(ClassicSuppressionProfile::Balanced)
        ? ClassicSuppressionProfile::Balanced
        : ClassicSuppressionProfile::MusicPreservation;
}

constexpr float MinimumCenterGain(ClassicSuppressionProfile profile) noexcept {
    return profile == ClassicSuppressionProfile::Balanced ? 0.12589255f : 0.25118864f;
}

constexpr float CenterBandGain(LONG vocalPercent, ClassicSuppressionProfile profile) noexcept {
    const LONG bounded = vocalPercent < 0 ? 0 : (vocalPercent > 100 ? 100 : vocalPercent);
    const float dry = static_cast<float>(bounded) / 100.0f;
    const float floor = MinimumCenterGain(profile);
    return floor + ((1.0f - floor) * dry);
}

constexpr float LowBandAlpha(ClassicSuppressionProfile profile) noexcept {
    // Approximate one-pole cutoffs at common Windows 44.1/48 kHz mix rates.
    return profile == ClassicSuppressionProfile::Balanced ? 0.0156f : 0.0233f;
}

constexpr float HighBandAlpha(ClassicSuppressionProfile profile) noexcept {
    return profile == ClassicSuppressionProfile::Balanced ? 0.6920f : 0.5440f;
}

} // namespace voxveil
