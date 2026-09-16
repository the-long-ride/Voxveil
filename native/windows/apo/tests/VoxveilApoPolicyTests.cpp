#include "../VoxveilApoPolicy.h"

#include <cassert>

int wmain() {
    using namespace voxveil;

    assert(ClassifyInitSize(sizeof(APOInitSystemEffects)) == ApoInitFlavor::V1);
    assert(ClassifyInitSize(sizeof(APOInitSystemEffects2)) == ApoInitFlavor::V2);
    assert(ClassifyInitSize(sizeof(APOInitSystemEffects3)) == ApoInitFlavor::V3);
    assert(ClassifyInitSize(1) == ApoInitFlavor::Invalid);

    assert(ShouldCountLoadedInstance(ApoInitFlavor::V1, false));
    assert(ShouldCountLoadedInstance(ApoInitFlavor::V2, false));
    assert(ShouldCountLoadedInstance(ApoInitFlavor::V3, false));
    assert(!ShouldCountLoadedInstance(ApoInitFlavor::V3, true));
    assert(!ShouldCountLoadedInstance(ApoInitFlavor::Invalid, false));

    assert(ShouldProcess(true, true, 50));
    assert(!ShouldProcess(false, true, 50));
    assert(!ShouldProcess(true, false, 50));
    assert(!ShouldProcess(true, true, 100));

    const auto music = ClassicSuppressionProfile::MusicPreservation;
    const auto balanced = ClassicSuppressionProfile::Balanced;
    assert(NormalizeProfile(999) == music);
    assert(NormalizeProfile(static_cast<LONG>(balanced)) == balanced);
    assert(MinimumCenterGain(music) > MinimumCenterGain(balanced));
    assert(MinimumCenterGain(balanced) > 0.0f);
    assert(CenterBandGain(0, music) == MinimumCenterGain(music));
    assert(CenterBandGain(0, balanced) == MinimumCenterGain(balanced));
    assert(CenterBandGain(100, music) == 1.0f);
    assert(CenterBandGain(100, balanced) == 1.0f);
    assert(LowBandAlpha(music) > LowBandAlpha(balanced));
    assert(HighBandAlpha(music) < HighBandAlpha(balanced));

    return 0;
}
