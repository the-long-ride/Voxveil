#pragma once

#include <atomic>
#include <cstdint>
#include <thread>

namespace voxveil {

class ControlState final {
public:
    ControlState();
    ~ControlState();

    ControlState(const ControlState&) = delete;
    ControlState& operator=(const ControlState&) = delete;

    bool enabled() const noexcept { return enabled_.load(std::memory_order_relaxed); }
    std::uint8_t vocal_level() const noexcept {
        return vocal_level_.load(std::memory_order_relaxed);
    }

private:
    void run();
    void reload() noexcept;

    std::atomic<bool> stop_{false};
    std::atomic<bool> enabled_{false};
    std::atomic<std::uint8_t> vocal_level_{100};
    std::thread worker_;
};

} // namespace voxveil
