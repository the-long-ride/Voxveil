#include "voxveil_dsp.h"

#include <algorithm>

namespace voxveil {

void process_interleaved(
    float* samples,
    std::uint32_t frame_count,
    std::uint32_t channels,
    bool enabled,
    std::uint8_t vocal_level) noexcept {
    if (samples == nullptr || !enabled || channels < 2 || frame_count == 0) {
        return;
    }

    const auto clamped = std::min<std::uint8_t>(vocal_level, 100);
    if (clamped == 100) {
        return;
    }

    const float level = static_cast<float>(clamped) / 100.0F;
    for (std::uint32_t frame = 0; frame < frame_count; ++frame) {
        float* current = samples + static_cast<std::size_t>(frame) * channels;
        const float left = current[0];
        const float right = current[1];
        const float mid = (left + right) * 0.5F;
        const float side = (left - right) * 0.5F;
        const float reduced_mid = mid * level;
        current[0] = reduced_mid + side;
        current[1] = reduced_mid - side;
    }
}

} // namespace voxveil
