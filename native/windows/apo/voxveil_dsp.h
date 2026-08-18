#pragma once

#include <cstdint>

namespace voxveil {

void process_interleaved(
    float* samples,
    std::uint32_t frame_count,
    std::uint32_t channels,
    bool enabled,
    std::uint8_t vocal_level) noexcept;

} // namespace voxveil
