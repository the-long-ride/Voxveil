#include "control_state.h"

#include <windows.h>

#include <array>
#include <chrono>
#include <filesystem>
#include <fstream>

namespace voxveil {
namespace {

std::filesystem::path control_path() {
    wchar_t buffer[32768]{};
    const DWORD length = GetEnvironmentVariableW(L"ProgramData", buffer, 32768);
    if (length > 0 && length < 32768) {
        return std::filesystem::path(buffer) / L"Voxveil" / L"apo-control.bin";
    }
    return std::filesystem::path(L"C:\\ProgramData\\Voxveil\\apo-control.bin");
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
    while (!stop_.load(std::memory_order_relaxed)) {
        reload();
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

} // namespace voxveil
