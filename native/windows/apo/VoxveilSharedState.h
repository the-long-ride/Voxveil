#pragma once

#include <windows.h>
#include <sddl.h>

namespace voxveil {

constexpr wchar_t kSharedStateName[] = L"Local\\VoxveilApoControl-v1";
constexpr LONG kSharedStateAbi = 1;

struct SharedState {
    volatile LONG abi;
    volatile LONG enabled;
    volatile LONG vocalPercent;
    volatile LONG heartbeat;
    volatile LONG loadedInstances;
};

inline SharedState* OpenOrCreateSharedState(HANDLE* mappingOut) noexcept {
    if (mappingOut == nullptr) {
        return nullptr;
    }

    PSECURITY_DESCRIPTOR descriptor = nullptr;
    SECURITY_ATTRIBUTES attributes{};
    attributes.nLength = sizeof(attributes);
    attributes.bInheritHandle = FALSE;

    if (ConvertStringSecurityDescriptorToSecurityDescriptorW(
            L"D:(A;;GA;;;AU)(A;;GA;;;SY)",
            SDDL_REVISION_1,
            &descriptor,
            nullptr)) {
        attributes.lpSecurityDescriptor = descriptor;
    }

    HANDLE mapping = CreateFileMappingW(
        INVALID_HANDLE_VALUE,
        attributes.lpSecurityDescriptor != nullptr ? &attributes : nullptr,
        PAGE_READWRITE,
        0,
        sizeof(SharedState),
        kSharedStateName);

    if (descriptor != nullptr) {
        LocalFree(descriptor);
    }
    if (mapping == nullptr) {
        return nullptr;
    }

    auto* state = static_cast<SharedState*>(
        MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, sizeof(SharedState)));
    if (state == nullptr) {
        CloseHandle(mapping);
        return nullptr;
    }

    if (InterlockedCompareExchange(&state->abi, kSharedStateAbi, 0) == 0) {
        InterlockedExchange(&state->enabled, 0);
        InterlockedExchange(&state->vocalPercent, 100);
        InterlockedExchange(&state->heartbeat, 0);
        InterlockedExchange(&state->loadedInstances, 0);
    }

    *mappingOut = mapping;
    return state;
}

inline void CloseSharedState(HANDLE mapping, SharedState* state) noexcept {
    if (state != nullptr) {
        UnmapViewOfFile(state);
    }
    if (mapping != nullptr) {
        CloseHandle(mapping);
    }
}

} // namespace voxveil
