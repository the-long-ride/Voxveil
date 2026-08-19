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
    void note_process() noexcept {
        process_count_.fetch_add(1, std::memory_order_relaxed);
    }

private:
    void run();
    void reload() noexcept;
    void write_runtime_heartbeat() noexcept;

    std::atomic<bool> stop_{false};
    std::atomic<bool> enabled_{false};
    std::atomic<std::uint8_t> vocal_level_{100};
    std::atomic<std::uint64_t> process_count_{0};
    std::thread worker_;
};

} // namespace voxveil
