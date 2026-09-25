#pragma once

#include <windows.h>
#include <sddl.h>

namespace voxveil {

constexpr wchar_t kSharedStateName[] = L"Local\\VoxveilApoControl-v4";
constexpr LONG kSharedStateAbi = 4;
constexpr LONG kSharedStateInitializing = -1;
constexpr LONG kMusicPreservationProfile = 0;
constexpr LONG kBalancedProfile = 1;
constexpr LONG kStrongProfile = 2;

struct SharedState {
    volatile LONG abi;
    volatile LONG enabled;
    volatile LONG systemEffectEnabled;
    volatile LONG vocalPercent;
    volatile LONG suppressionProfile;
    volatile LONG heartbeat;
    volatile LONG loadedInstances;
    volatile LONG capxInstances;
};

inline SharedState* OpenOrCreateSharedState(HANDLE* mappingOut) noexcept {
    if (mappingOut == nullptr) {
        return nullptr;
    }

    *mappingOut = nullptr;

    PSECURITY_DESCRIPTOR descriptor = nullptr;
    SECURITY_ATTRIBUTES attributes{};
    attributes.nLength = sizeof(attributes);
    attributes.bInheritHandle = FALSE;

    if (ConvertStringSecurityDescriptorToSecurityDescriptorW(
            L"D:(A;;GA;;;AU)(A;;GA;;;LS)(A;;GA;;;SY)",
            SDDL_REVISION_1,
            &descriptor,
            nullptr)) {
        attributes.lpSecurityDescriptor = descriptor;
    }

    SetLastError(ERROR_SUCCESS);
    HANDLE mapping = CreateFileMappingW(
        INVALID_HANDLE_VALUE,
        attributes.lpSecurityDescriptor != nullptr ? &attributes : nullptr,
        PAGE_READWRITE,
        0,
        sizeof(SharedState),
        kSharedStateName);
    const DWORD createError = GetLastError();

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

    if (createError != ERROR_ALREADY_EXISTS) {
        InterlockedExchange(&state->abi, kSharedStateInitializing);
        InterlockedExchange(&state->enabled, 0);
        InterlockedExchange(&state->systemEffectEnabled, 1);
        InterlockedExchange(&state->vocalPercent, 100);
        InterlockedExchange(&state->suppressionProfile, kMusicPreservationProfile);
        InterlockedExchange(&state->heartbeat, 0);
        InterlockedExchange(&state->loadedInstances, 0);
        InterlockedExchange(&state->capxInstances, 0);
        MemoryBarrier();
        InterlockedExchange(&state->abi, kSharedStateAbi);
    } else {
        LONG observedAbi = InterlockedCompareExchange(&state->abi, 0, 0);
        for (unsigned int attempt = 0;
             attempt < 100 && (observedAbi == 0 || observedAbi == kSharedStateInitializing);
             ++attempt) {
            Sleep(1);
            observedAbi = InterlockedCompareExchange(&state->abi, 0, 0);
        }
        if (observedAbi != kSharedStateAbi) {
            UnmapViewOfFile(state);
            CloseHandle(mapping);
            SetLastError(ERROR_REVISION_MISMATCH);
            return nullptr;
        }
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
