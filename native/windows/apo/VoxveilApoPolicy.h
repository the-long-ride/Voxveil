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

} // namespace voxveil
