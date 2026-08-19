#include "control_state.h"

#include <windows.h>

#include <array>
#include <chrono>
#include <filesystem>
#include <fstream>

namespace voxveil {
namespace {

std::filesystem::path state_path(const wchar_t* name) {
    wchar_t buffer[32768]{};
    const DWORD length = GetEnvironmentVariableW(L"ProgramData", buffer, 32768);
    const auto root = (length > 0 && length < 32768)
                          ? std::filesystem::path(buffer)
                          : std::filesystem::path(L"C:\\ProgramData");
    return root / L"Voxveil" / name;
}

std::filesystem::path control_path() {
    return state_path(L"apo-control.bin");
}

std::filesystem::path runtime_path() {
    return state_path(L"apo-runtime.bin");
}

template <typename T>
void write_little_endian(std::array<unsigned char, 21>& bytes, std::size_t offset, T value) {
    for (std::size_t index = 0; index < sizeof(T); ++index) {
        bytes[offset + index] = static_cast<unsigned char>((value >> (index * 8)) & 0xff);
    }
}

} // namespace

ControlState::ControlState() : worker_([this] { run(); }) {}

ControlState::~ControlState() {
    stop_.store(true, std::memory_order_relaxed);
    if (worker_.joinable()) {
        worker_.join();
    }
}

void ControlState::run() {
    unsigned heartbeat_tick = 0;
    while (!stop_.load(std::memory_order_relaxed)) {
        reload();
        if (++heartbeat_tick >= 5) {
            heartbeat_tick = 0;
            write_runtime_heartbeat();
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
    }
}

void ControlState::reload() noexcept {
    try {
        std::ifstream input(control_path(), std::ios::binary);
        std::array<unsigned char, 3> bytes{};
        if (!input.read(reinterpret_cast<char*>(bytes.data()), bytes.size())) {
            return;
        }
        if (bytes[0] != 1) {
            return;
        }
        enabled_.store(bytes[1] != 0, std::memory_order_relaxed);
        vocal_level_.store(bytes[2] > 100 ? 100 : bytes[2], std::memory_order_relaxed);
    } catch (...) {
        // Fail open: keep the last known controls and never disturb audio processing.
    }
}

void ControlState::write_runtime_heartbeat() noexcept {
    try {
        const auto now = std::chrono::duration_cast<std::chrono::milliseconds>(
                             std::chrono::system_clock::now().time_since_epoch())
                             .count();
        std::array<unsigned char, 21> bytes{};
        bytes[0] = 1;
        write_little_endian(bytes, 1, static_cast<std::uint64_t>(now));
        write_little_endian(bytes, 9, process_count_.load(std::memory_order_relaxed));
        write_little_endian(bytes, 17, static_cast<std::uint32_t>(GetCurrentProcessId()));
        std::ofstream output(runtime_path(), std::ios::binary | std::ios::trunc);
        output.write(reinterpret_cast<const char*>(bytes.data()), bytes.size());
    } catch (...) {
        // Diagnostics must never affect the audio process.
    }
}

} // namespace voxveil
