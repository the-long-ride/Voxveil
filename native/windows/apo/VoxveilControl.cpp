#include <windows.h>

#include "VoxveilSharedState.h"

namespace {

int WithState(int (*operation)(voxveil::SharedState*)) noexcept {
    HANDLE mapping = nullptr;
    voxveil::SharedState* state = voxveil::OpenOrCreateSharedState(&mapping);
    if (state == nullptr) {
        return static_cast<int>(GetLastError() == ERROR_SUCCESS ? ERROR_OPEN_FAILED : GetLastError());
    }
    const int result = operation(state);
    voxveil::CloseSharedState(mapping, state);
    return result;
}

} // namespace

extern "C" __declspec(dllexport) int __stdcall VoxveilSetEnabled(int enabled) noexcept {
    return WithState([enabled](voxveil::SharedState* state) noexcept -> int {
        InterlockedExchange(&state->enabled, enabled != 0 ? 1 : 0);
        return ERROR_SUCCESS;
    });
}

extern "C" __declspec(dllexport) int __stdcall VoxveilSetVocalLevel(unsigned int percent) noexcept {
    if (percent > 100) {
        return ERROR_INVALID_PARAMETER;
    }
    return WithState([percent](voxveil::SharedState* state) noexcept -> int {
        InterlockedExchange(&state->vocalPercent, static_cast<LONG>(percent));
        return ERROR_SUCCESS;
    });
}

extern "C" __declspec(dllexport) int __stdcall VoxveilGetState(
    int* enabled,
    unsigned int* vocalPercent,
    unsigned int* heartbeat,
    unsigned int* loadedInstances) noexcept {
    if (enabled == nullptr || vocalPercent == nullptr || heartbeat == nullptr || loadedInstances == nullptr) {
        return ERROR_INVALID_PARAMETER;
    }
    HANDLE mapping = nullptr;
    voxveil::SharedState* state = voxveil::OpenOrCreateSharedState(&mapping);
    if (state == nullptr) {
        return static_cast<int>(GetLastError() == ERROR_SUCCESS ? ERROR_OPEN_FAILED : GetLastError());
    }
    *enabled = InterlockedCompareExchange(&state->enabled, 0, 0) != 0 ? 1 : 0;
    *vocalPercent = static_cast<unsigned int>(InterlockedCompareExchange(&state->vocalPercent, 0, 0));
    *heartbeat = static_cast<unsigned int>(InterlockedCompareExchange(&state->heartbeat, 0, 0));
    *loadedInstances = static_cast<unsigned int>(InterlockedCompareExchange(&state->loadedInstances, 0, 0));
    voxveil::CloseSharedState(mapping, state);
    return ERROR_SUCCESS;
}
