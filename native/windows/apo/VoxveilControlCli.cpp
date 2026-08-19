#include <windows.h>

#include <filesystem>
#include <iostream>
#include <string>

namespace {

using SetEnabledFn = int(__stdcall*)(int);
using SetVocalFn = int(__stdcall*)(unsigned int);
using GetStateFn = int(__stdcall*)(int*, unsigned int*, unsigned int*, unsigned int*);

std::filesystem::path ControlDllPath() {
    wchar_t buffer[MAX_PATH]{};
    const DWORD length = GetModuleFileNameW(nullptr, buffer, MAX_PATH);
    if (length == 0 || length >= MAX_PATH) {
        return L"VoxveilControl.dll";
    }
    return std::filesystem::path(buffer).parent_path() / L"VoxveilControl.dll";
}

int ParsePercent(const wchar_t* text, unsigned int* value) {
    try {
        const unsigned long parsed = std::stoul(text);
        if (parsed > 100) {
            return ERROR_INVALID_PARAMETER;
        }
        *value = static_cast<unsigned int>(parsed);
        return ERROR_SUCCESS;
    } catch (...) {
        return ERROR_INVALID_PARAMETER;
    }
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    HMODULE module = LoadLibraryW(ControlDllPath().c_str());
    if (module == nullptr) {
        std::wcerr << L"failed to load VoxveilControl.dll: " << GetLastError() << L'\n';
        return 2;
    }

    auto setEnabled = reinterpret_cast<SetEnabledFn>(GetProcAddress(module, "VoxveilSetEnabled"));
    auto setVocal = reinterpret_cast<SetVocalFn>(GetProcAddress(module, "VoxveilSetVocalLevel"));
    auto getState = reinterpret_cast<GetStateFn>(GetProcAddress(module, "VoxveilGetState"));
    if (setEnabled == nullptr || setVocal == nullptr || getState == nullptr) {
        std::wcerr << L"VoxveilControl.dll is missing required exports\n";
        FreeLibrary(module);
        return 3;
    }

    int result = ERROR_INVALID_PARAMETER;
    if (argc == 2 && std::wstring(argv[1]) == L"status") {
        int enabled = 0;
        unsigned int vocal = 0;
        unsigned int heartbeat = 0;
        unsigned int loaded = 0;
        result = getState(&enabled, &vocal, &heartbeat, &loaded);
        if (result == ERROR_SUCCESS) {
            std::wcout << L"enabled=" << enabled
                       << L" vocal=" << vocal
                       << L" heartbeat=" << heartbeat
                       << L" loaded=" << loaded << L'\n';
        }
    } else if (argc == 3 && std::wstring(argv[1]) == L"enabled") {
        const std::wstring value(argv[2]);
        if (value == L"0" || value == L"1") {
            result = setEnabled(value == L"1" ? 1 : 0);
        }
    } else if (argc == 3 && std::wstring(argv[1]) == L"vocal") {
        unsigned int percent = 0;
        result = ParsePercent(argv[2], &percent);
        if (result == ERROR_SUCCESS) {
            result = setVocal(percent);
        }
    } else {
        std::wcerr << L"usage: voxveil-control status | enabled <0|1> | vocal <0..100>\n";
    }

    if (result != ERROR_SUCCESS) {
        std::wcerr << L"control operation failed: " << result << L'\n';
    }
    FreeLibrary(module);
    return result == ERROR_SUCCESS ? 0 : 1;
}
