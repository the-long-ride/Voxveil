#include "voxveil_dsp.h"

#include <array>
#include <cmath>
#include <iostream>

namespace {

bool near(float actual, float expected) {
    return std::fabs(actual - expected) < 1.0e-6F;
}

int check(bool condition, const char* message) {
    if (!condition) {
        std::cerr << "FAIL: " << message << '\n';
        return 1;
    }
    return 0;
}

int disabled_is_passthrough() {
    std::array<float, 4> samples{0.25F, -0.5F, 0.7F, 0.7F};
    const auto original = samples;
    voxveil::process_interleaved(samples.data(), 2, 2, false, 0);
    return check(samples == original, "disabled processing must pass through");
}

int level_100_is_passthrough() {
    std::array<float, 4> samples{0.25F, -0.5F, 0.7F, 0.7F};
    const auto original = samples;
    voxveil::process_interleaved(samples.data(), 2, 2, true, 100);
    return check(samples == original, "100 percent vocal level must pass through");
}

int level_zero_removes_center() {
    std::array<float, 4> samples{0.8F, 0.8F, -0.4F, -0.4F};
    voxveil::process_interleaved(samples.data(), 2, 2, true, 0);
    for (const float sample : samples) {
        if (!near(sample, 0.0F)) {
            return check(false, "zero vocal level must remove centered stereo signal");
        }
    }
    return 0;
}

int side_information_survives() {
    std::array<float, 2> samples{0.6F, -0.6F};
    voxveil::process_interleaved(samples.data(), 1, 2, true, 0);
    return check(
        near(samples[0], 0.6F) && near(samples[1], -0.6F),
        "pure side information must survive");
}

int extra_channels_are_untouched() {
    std::array<float, 4> samples{0.8F, 0.8F, 0.25F, -0.25F};
    voxveil::process_interleaved(samples.data(), 1, 4, true, 0);
    return check(
        near(samples[0], 0.0F) && near(samples[1], 0.0F) &&
            near(samples[2], 0.25F) && near(samples[3], -0.25F),
        "channels after L/R must pass through");
}

} // namespace

int main() {
    int failures = 0;
    failures += disabled_is_passthrough();
    failures += level_100_is_passthrough();
    failures += level_zero_removes_center();
    failures += side_information_survives();
    failures += extra_channels_are_untouched();
    if (failures == 0) {
        std::cout << "Voxveil native DSP tests passed\n";
    }
    return failures == 0 ? 0 : 1;
}
