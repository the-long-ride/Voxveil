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

    return 0;
}
