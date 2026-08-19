#include <windows.h>

#include "VoxveilSharedState.h"

namespace {

int OpenState(HANDLE* mapping, voxveil::SharedState** state) noexcept {
    *mapping = nullptr;
    *state = voxveil::OpenOrCreateSharedState(mapping);
    if (*state == nullptr) {
        const DWORD error = GetLastError();
        return static_cast<int>(error == ERROR_SUCCESS ? ERROR_OPEN_FAILED : error);
    }
    return ERROR_SUCCESS;
}

} // namespace

extern "C" __declspec(dllexport) int __stdcall VoxveilSetEnabled(int enabled) noexcept {
    HANDLE mapping = nullptr;
    voxveil::SharedState* state = nullptr;
    const int error = OpenState(&mapping, &state);
    if (error != ERROR_SUCCESS) {
        return error;
    }
    InterlockedExchange(&state->enabled, enabled != 0 ? 1 : 0);
    voxveil::CloseSharedState(mapping, state);
    return ERROR_SUCCESS;
}

extern "C" __declspec(dllexport) int __stdcall VoxveilSetVocalLevel(unsigned int percent) noexcept {
    if (percent > 100) {
        return ERROR_INVALID_PARAMETER;
    }
    HANDLE mapping = nullptr;
    voxveil::SharedState* state = nullptr;
    const int error = OpenState(&mapping, &state);
    if (error != ERROR_SUCCESS) {
        return error;
    }
    InterlockedExchange(&state->vocalPercent, static_cast<LONG>(percent));
    voxveil::CloseSharedState(mapping, state);
    return ERROR_SUCCESS;
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
    voxveil::SharedState* state = nullptr;
    const int error = OpenState(&mapping, &state);
    if (error != ERROR_SUCCESS) {
        return error;
    }

    *enabled = InterlockedCompareExchange(&state->enabled, 0, 0) != 0 ? 1 : 0;
    *vocalPercent = static_cast<unsigned int>(InterlockedCompareExchange(&state->vocalPercent, 0, 0));
    *heartbeat = static_cast<unsigned int>(InterlockedCompareExchange(&state->heartbeat, 0, 0));
    *loadedInstances = static_cast<unsigned int>(InterlockedCompareExchange(&state->loadedInstances, 0, 0));
    voxveil::CloseSharedState(mapping, state);
    return ERROR_SUCCESS;
}
