#include <windows.h>
#include <setupapi.h>

#include <algorithm>
#include <cwchar>
#include <filesystem>
#include <iostream>
#include <string>
#include <vector>

namespace {

using SetEnabledFn = int(__stdcall*)(int);
using SetVocalFn = int(__stdcall*)(unsigned int);
using GetStateFn = int(__stdcall*)(int*, unsigned int*, unsigned int*, unsigned int*);

constexpr wchar_t kFxKey[] = L"FX\\0";
constexpr wchar_t kFxAssociation[] = L"{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},0";
constexpr wchar_t kCompositeSfx[] = L"{D04E05A6-594B-4fb6-A80D-01AF5EED7D1D},13";
constexpr wchar_t kSfxModes[] = L"{D3993A3F-99C2-4402-B5EC-A92A0367664B},5";
constexpr wchar_t kVoxveilSfxClsid[] = L"{F3F2A99F-8FB7-4B88-949E-448BF8A05221}";
constexpr wchar_t kKsNodeTypeAny[] = L"{00000000-0000-0000-0000-000000000000}";
constexpr wchar_t kModeDefault[] = L"{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}";
constexpr wchar_t kModeMedia[] = L"{4780004E-7133-41D8-8C74-660DADD2C0EE}";
constexpr wchar_t kModeMovie[] = L"{B26FEB0D-EC94-477C-9494-D1AB8E753F6E}";

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

int ReadDeviceInstanceId(HDEVINFO set, SP_DEVINFO_DATA& deviceInfo, std::wstring* value) {
    DWORD required = 0;
    SetupDiGetDeviceInstanceIdW(set, &deviceInfo, nullptr, 0, &required);
    if (required == 0) {
        return static_cast<int>(GetLastError());
    }
    std::vector<wchar_t> buffer(required);
    if (!SetupDiGetDeviceInstanceIdW(set, &deviceInfo, buffer.data(), required, nullptr)) {
        return static_cast<int>(GetLastError());
    }
    *value = buffer.data();
    return ERROR_SUCCESS;
}

int OpenInterfaceFxKey(
    const wchar_t* expectedInstanceId,
    const wchar_t* interfacePath,
    bool create,
    HKEY* fxKey) {
    *fxKey = nullptr;
    HDEVINFO set = SetupDiCreateDeviceInfoList(nullptr, nullptr);
    if (set == INVALID_HANDLE_VALUE) {
        return static_cast<int>(GetLastError());
    }

    SP_DEVICE_INTERFACE_DATA interfaceData{};
    interfaceData.cbSize = sizeof(interfaceData);
    if (!SetupDiOpenDeviceInterfaceW(set, interfacePath, 0, &interfaceData)) {
        const int error = static_cast<int>(GetLastError());
        SetupDiDestroyDeviceInfoList(set);
        return error;
    }

    DWORD required = 0;
    SetupDiGetDeviceInterfaceDetailW(set, &interfaceData, nullptr, 0, &required, nullptr);
    if (required == 0) {
        const int error = static_cast<int>(GetLastError());
        SetupDiDestroyDeviceInfoList(set);
        return error;
    }

    std::vector<BYTE> detailBuffer(required);
    auto* detail = reinterpret_cast<SP_DEVICE_INTERFACE_DETAIL_DATA_W*>(detailBuffer.data());
    detail->cbSize = sizeof(SP_DEVICE_INTERFACE_DETAIL_DATA_W);
    SP_DEVINFO_DATA deviceInfo{};
    deviceInfo.cbSize = sizeof(deviceInfo);
    if (!SetupDiGetDeviceInterfaceDetailW(
            set,
            &interfaceData,
            detail,
            required,
            nullptr,
            &deviceInfo)) {
        const int error = static_cast<int>(GetLastError());
        SetupDiDestroyDeviceInfoList(set);
        return error;
    }

    std::wstring actualInstanceId;
    int result = ReadDeviceInstanceId(set, deviceInfo, &actualInstanceId);
    if (result != ERROR_SUCCESS) {
        SetupDiDestroyDeviceInfoList(set);
        return result;
    }
    if (_wcsicmp(actualInstanceId.c_str(), expectedInstanceId) != 0) {
        SetupDiDestroyDeviceInfoList(set);
        return ERROR_DEVICE_NOT_CONNECTED;
    }

    HKEY interfaceKey = SetupDiOpenDeviceInterfaceRegKey(
        set,
        &interfaceData,
        0,
        KEY_READ | KEY_WRITE);
    if (interfaceKey == INVALID_HANDLE_VALUE) {
        const int error = static_cast<int>(GetLastError());
        SetupDiDestroyDeviceInfoList(set);
        return error;
    }

    HKEY openedFxKey = nullptr;
    LSTATUS registryResult = ERROR_SUCCESS;
    if (create) {
        registryResult = RegCreateKeyExW(
            interfaceKey,
            kFxKey,
            0,
            nullptr,
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            nullptr,
            &openedFxKey,
            nullptr);
    } else {
        registryResult = RegOpenKeyExW(interfaceKey, kFxKey, 0, KEY_READ | KEY_WRITE, &openedFxKey);
    }

    RegCloseKey(interfaceKey);
    SetupDiDestroyDeviceInfoList(set);
    if (registryResult != ERROR_SUCCESS) {
        return static_cast<int>(registryResult);
    }
    *fxKey = openedFxKey;
    return ERROR_SUCCESS;
}

bool EqualsIgnoreCase(const std::wstring& left, const std::wstring& right) {
    return _wcsicmp(left.c_str(), right.c_str()) == 0;
}

int ReadMultiSz(HKEY key, const wchar_t* name, std::vector<std::wstring>* values) {
    values->clear();
    DWORD type = 0;
    DWORD bytes = 0;
    LSTATUS result = RegQueryValueExW(key, name, nullptr, &type, nullptr, &bytes);
    if (result == ERROR_FILE_NOT_FOUND) {
        return ERROR_SUCCESS;
    }
    if (result != ERROR_SUCCESS) {
        return static_cast<int>(result);
    }
    if (type != REG_MULTI_SZ) {
        return ERROR_DATATYPE_MISMATCH;
    }
    if (bytes == 0) {
        return ERROR_SUCCESS;
    }

    std::vector<wchar_t> buffer((bytes / sizeof(wchar_t)) + 2, L'\0');
    result = RegQueryValueExW(
        key,
        name,
        nullptr,
        &type,
        reinterpret_cast<BYTE*>(buffer.data()),
        &bytes);
    if (result != ERROR_SUCCESS) {
        return static_cast<int>(result);
    }

    const wchar_t* current = buffer.data();
    const wchar_t* end = buffer.data() + buffer.size();
    while (current < end && *current != L'\0') {
        std::wstring value(current);
        values->push_back(value);
        current += value.size() + 1;
    }
    return ERROR_SUCCESS;
}

int WriteMultiSz(HKEY key, const wchar_t* name, const std::vector<std::wstring>& values) {
    if (values.empty()) {
        const LSTATUS result = RegDeleteValueW(key, name);
        return result == ERROR_FILE_NOT_FOUND ? ERROR_SUCCESS : static_cast<int>(result);
    }

    std::vector<wchar_t> buffer;
    for (const auto& value : values) {
        buffer.insert(buffer.end(), value.begin(), value.end());
        buffer.push_back(L'\0');
    }
    buffer.push_back(L'\0');
    return static_cast<int>(RegSetValueExW(
        key,
        name,
        0,
        REG_MULTI_SZ,
        reinterpret_cast<const BYTE*>(buffer.data()),
        static_cast<DWORD>(buffer.size() * sizeof(wchar_t))));
}

int AppendMultiSz(HKEY key, const wchar_t* name, const std::vector<std::wstring>& additions) {
    std::vector<std::wstring> values;
    int result = ReadMultiSz(key, name, &values);
    if (result != ERROR_SUCCESS) {
        return result;
    }
    for (const auto& addition : additions) {
        if (std::none_of(values.begin(), values.end(), [&](const std::wstring& value) {
                return EqualsIgnoreCase(value, addition);
            })) {
            values.push_back(addition);
        }
    }
    return WriteMultiSz(key, name, values);
}

int RemoveMultiSz(HKEY key, const wchar_t* name, const std::wstring& removal) {
    std::vector<std::wstring> values;
    int result = ReadMultiSz(key, name, &values);
    if (result != ERROR_SUCCESS) {
        return result;
    }
    values.erase(
        std::remove_if(values.begin(), values.end(), [&](const std::wstring& value) {
            return EqualsIgnoreCase(value, removal);
        }),
        values.end());
    return WriteMultiSz(key, name, values);
}

int EnsureAssociation(HKEY key) {
    DWORD type = 0;
    DWORD bytes = 0;
    const LSTATUS query = RegQueryValueExW(key, kFxAssociation, nullptr, &type, nullptr, &bytes);
    if (query != ERROR_FILE_NOT_FOUND) {
        return query == ERROR_SUCCESS ? ERROR_SUCCESS : static_cast<int>(query);
    }
    return static_cast<int>(RegSetValueExW(
        key,
        kFxAssociation,
        0,
        REG_SZ,
        reinterpret_cast<const BYTE*>(kKsNodeTypeAny),
        static_cast<DWORD>(sizeof(kKsNodeTypeAny))));
}

int ApplyEffects(HKEY key) {
    int result = EnsureAssociation(key);
    if (result != ERROR_SUCCESS) {
        return result;
    }
    result = AppendMultiSz(key, kCompositeSfx, {kVoxveilSfxClsid});
    if (result != ERROR_SUCCESS) {
        return result;
    }
    return AppendMultiSz(key, kSfxModes, {kModeDefault, kModeMedia, kModeMovie});
}

int RemoveEffects(HKEY key) {
    return RemoveMultiSz(key, kCompositeSfx, kVoxveilSfxClsid);
}

int MutateRuntimeInterfaces(
    bool attach,
    const wchar_t* expectedInstanceId,
    const wchar_t* topologyPath,
    const wchar_t* audioPath) {
    HKEY topologyKey = nullptr;
    HKEY audioKey = nullptr;
    int result = OpenInterfaceFxKey(expectedInstanceId, topologyPath, attach, &topologyKey);
    if (!attach && result == ERROR_FILE_NOT_FOUND) {
        result = ERROR_SUCCESS;
    }
    if (result != ERROR_SUCCESS) {
        return result;
    }

    result = OpenInterfaceFxKey(expectedInstanceId, audioPath, attach, &audioKey);
    if (!attach && result == ERROR_FILE_NOT_FOUND) {
        result = ERROR_SUCCESS;
    }
    if (result != ERROR_SUCCESS) {
        if (topologyKey != nullptr) {
            RegCloseKey(topologyKey);
        }
        return result;
    }

    if (attach) {
        result = ApplyEffects(topologyKey);
        if (result == ERROR_SUCCESS) {
            result = ApplyEffects(audioKey);
        }
        if (result != ERROR_SUCCESS) {
            RemoveEffects(topologyKey);
            RemoveEffects(audioKey);
        }
    } else {
        if (topologyKey != nullptr) {
            result = RemoveEffects(topologyKey);
        }
        if (result == ERROR_SUCCESS && audioKey != nullptr) {
            result = RemoveEffects(audioKey);
        }
    }

    if (topologyKey != nullptr) {
        RegCloseKey(topologyKey);
    }
    if (audioKey != nullptr) {
        RegCloseKey(audioKey);
    }
    return result;
}

void PrintUsage() {
    std::wcerr
        << L"usage: voxveil-control status | enabled <0|1> | vocal <0..100> | "
        << L"attach-effects <binding-instance-id> <topology-interface-path> <audio-interface-path> | "
        << L"detach-effects <binding-instance-id> <topology-interface-path> <audio-interface-path>\n";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc == 5 && std::wstring(argv[1]) == L"attach-effects") {
        const int result = MutateRuntimeInterfaces(true, argv[2], argv[3], argv[4]);
        if (result != ERROR_SUCCESS) {
            std::wcerr << L"runtime interface attachment failed: " << result << L'\n';
        }
        return result == ERROR_SUCCESS ? 0 : 1;
    }
    if (argc == 5 && std::wstring(argv[1]) == L"detach-effects") {
        const int result = MutateRuntimeInterfaces(false, argv[2], argv[3], argv[4]);
        if (result != ERROR_SUCCESS) {
            std::wcerr << L"runtime interface detach failed: " << result << L'\n';
        }
        return result == ERROR_SUCCESS ? 0 : 1;
    }

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
        PrintUsage();
    }

    if (result != ERROR_SUCCESS) {
        std::wcerr << L"control operation failed: " << result << L'\n';
    }
    FreeLibrary(module);
    return result == ERROR_SUCCESS ? 0 : 1;
}
