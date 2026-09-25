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
    Strong = 2,
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
    if (value == static_cast<LONG>(ClassicSuppressionProfile::Strong)) {
        return ClassicSuppressionProfile::Strong;
    }
    if (value == static_cast<LONG>(ClassicSuppressionProfile::Balanced)) {
        return ClassicSuppressionProfile::Balanced;
    }
    return ClassicSuppressionProfile::MusicPreservation;
}

constexpr float MinimumCenterGain(
    ClassicSuppressionProfile profile,
    float sideToMidPowerRatio = 1.0f) noexcept {
    if (profile == ClassicSuppressionProfile::Strong) {
        const float boundedRatio = sideToMidPowerRatio < 0.0f
            ? 0.0f
            : (sideToMidPowerRatio > 1.0f ? 1.0f : sideToMidPowerRatio);
        const float confidence = boundedRatio <= 0.001f
            ? 0.0f
            : (boundedRatio >= 0.01f ? 1.0f : (boundedRatio - 0.001f) / 0.009f);
        return 0.5f + confidence * (0.03162278f - 0.5f);
    }
    return profile == ClassicSuppressionProfile::Balanced ? 0.12589255f : 0.25118864f;
}

constexpr float CenterBandGain(
    LONG vocalPercent,
    ClassicSuppressionProfile profile,
    float sideToMidPowerRatio = 1.0f) noexcept {
    const LONG bounded = vocalPercent < 0 ? 0 : (vocalPercent > 100 ? 100 : vocalPercent);
    const float dry = static_cast<float>(bounded) / 100.0f;
    const float floor = MinimumCenterGain(profile, sideToMidPowerRatio);
    return floor + ((1.0f - floor) * dry);
}

constexpr float LowBandAlpha(ClassicSuppressionProfile profile) noexcept {
    // Approximate one-pole cutoffs at common Windows 44.1/48 kHz mix rates.
    return profile == ClassicSuppressionProfile::MusicPreservation ? 0.0233f : 0.0156f;
}

constexpr float HighBandAlpha(ClassicSuppressionProfile profile) noexcept {
    return profile == ClassicSuppressionProfile::MusicPreservation ? 0.5440f : 0.6920f;
}

} // namespace voxveil
