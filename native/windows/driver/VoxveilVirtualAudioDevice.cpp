#include <windows.h>
#include <cfgmgr32.h>
#include <setupapi.h>

#include <algorithm>
#include <cwchar>
#include <iostream>
#include <iterator>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

constexpr wchar_t kHardwareId[] = L"Root\\VoxveilVirtualAudio";

class DeviceInfoSet final {
public:
    explicit DeviceInfoSet(HDEVINFO handle) : handle_(handle) {
        if (handle_ == INVALID_HANDLE_VALUE) {
            throw std::runtime_error("SetupAPI device-info set creation failed with Win32 error " +
                                     std::to_string(GetLastError()));
        }
    }

    DeviceInfoSet(const DeviceInfoSet&) = delete;
    DeviceInfoSet& operator=(const DeviceInfoSet&) = delete;

    ~DeviceInfoSet() {
        if (handle_ != INVALID_HANDLE_VALUE) {
            SetupDiDestroyDeviceInfoList(handle_);
        }
    }

    HDEVINFO get() const noexcept { return handle_; }

private:
    HDEVINFO handle_ = INVALID_HANDLE_VALUE;
};

[[noreturn]] void ThrowLastError(const char* operation) {
    const DWORD error = GetLastError();
    throw std::runtime_error(std::string(operation) + " failed with Win32 error " +
                             std::to_string(error));
}

std::wstring GetDeviceInstanceId(HDEVINFO set, SP_DEVINFO_DATA* device) {
    DWORD required = 0;
    if (SetupDiGetDeviceInstanceIdW(set, device, nullptr, 0, &required)) {
        throw std::runtime_error("SetupDiGetDeviceInstanceIdW unexpectedly returned an empty ID");
    }
    if (GetLastError() != ERROR_INSUFFICIENT_BUFFER || required == 0) {
        ThrowLastError("SetupDiGetDeviceInstanceIdW(size)");
    }

    std::vector<wchar_t> buffer(static_cast<size_t>(required) + 1, L'\0');
    if (!SetupDiGetDeviceInstanceIdW(
            set,
            device,
            buffer.data(),
            static_cast<DWORD>(buffer.size()),
            nullptr)) {
        ThrowLastError("SetupDiGetDeviceInstanceIdW");
    }
    return std::wstring(buffer.data());
}

bool HasExactHardwareId(HDEVINFO set, SP_DEVINFO_DATA* device) {
    DWORD propertyType = 0;
    DWORD required = 0;
    if (SetupDiGetDeviceRegistryPropertyW(
            set,
            device,
            SPDRP_HARDWAREID,
            &propertyType,
            nullptr,
            0,
            &required)) {
        return false;
    }

    const DWORD sizeError = GetLastError();
    if (sizeError == ERROR_INVALID_DATA) {
        return false;
    }
    if (sizeError != ERROR_INSUFFICIENT_BUFFER || required < sizeof(wchar_t) * 2) {
        ThrowLastError("SetupDiGetDeviceRegistryPropertyW(size)");
    }

    std::vector<BYTE> buffer(required, 0);
    if (!SetupDiGetDeviceRegistryPropertyW(
            set,
            device,
            SPDRP_HARDWAREID,
            &propertyType,
            buffer.data(),
            static_cast<DWORD>(buffer.size()),
            nullptr)) {
        ThrowLastError("SetupDiGetDeviceRegistryPropertyW");
    }
    if (propertyType != REG_MULTI_SZ) {
        return false;
    }

    const auto* item = reinterpret_cast<const wchar_t*>(buffer.data());
    while (*item != L'\0') {
        if (_wcsicmp(item, kHardwareId) == 0) {
            return true;
        }
        item += std::wcslen(item) + 1;
    }
    return false;
}

std::vector<std::wstring> FindMatchingDeviceInstanceIds() {
    DeviceInfoSet set(SetupDiGetClassDevsW(nullptr, nullptr, nullptr, DIGCF_ALLCLASSES));
    std::vector<std::wstring> matches;

    for (DWORD index = 0;; ++index) {
        SP_DEVINFO_DATA device{};
        device.cbSize = sizeof(device);
        if (!SetupDiEnumDeviceInfo(set.get(), index, &device)) {
            const DWORD error = GetLastError();
            if (error == ERROR_NO_MORE_ITEMS) {
                break;
            }
            ThrowLastError("SetupDiEnumDeviceInfo");
        }
        if (HasExactHardwareId(set.get(), &device)) {
            matches.push_back(GetDeviceInstanceId(set.get(), &device));
        }
    }
    return matches;
}

void SetExactHardwareId(HDEVINFO set, SP_DEVINFO_DATA* device) {
    const size_t hardwareIdChars = std::wcslen(kHardwareId);
    std::vector<wchar_t> multiSz(hardwareIdChars + 2, L'\0');
    std::copy_n(kHardwareId, hardwareIdChars, multiSz.data());

    const size_t byteCount = multiSz.size() * sizeof(wchar_t);
    if (byteCount > MAXDWORD) {
        throw std::runtime_error("hardware ID buffer is unexpectedly too large");
    }
    if (!SetupDiSetDeviceRegistryPropertyW(
            set,
            device,
            SPDRP_HARDWAREID,
            reinterpret_cast<const BYTE*>(multiSz.data()),
            static_cast<DWORD>(byteCount))) {
        ThrowLastError("SetupDiSetDeviceRegistryPropertyW(SPDRP_HARDWAREID)");
    }
}

bool DeviceInstallNeedsRestart(HDEVINFO set, SP_DEVINFO_DATA* device) {
    SP_DEVINSTALL_PARAMS_W params{};
    params.cbSize = sizeof(params);
    if (!SetupDiGetDeviceInstallParamsW(set, device, &params)) {
        ThrowLastError("SetupDiGetDeviceInstallParamsW");
    }
    return (params.Flags & (DI_NEEDREBOOT | DI_NEEDRESTART)) != 0;
}

struct CreateResult {
    std::wstring instanceId;
    bool rebootRequired = false;
};

CreateResult CreateRootDevice(const std::wstring& infPath) {
    GUID classGuid{};
    wchar_t className[MAX_CLASS_NAME_LEN]{};
    if (!SetupDiGetINFClassW(
            infPath.c_str(),
            &classGuid,
            className,
            static_cast<DWORD>(std::size(className)),
            nullptr)) {
        ThrowLastError("SetupDiGetINFClassW");
    }

    DeviceInfoSet set(SetupDiCreateDeviceInfoList(&classGuid, nullptr));
    SP_DEVINFO_DATA device{};
    device.cbSize = sizeof(device);
    if (!SetupDiCreateDeviceInfoW(
            set.get(),
            className,
            &classGuid,
            nullptr,
            nullptr,
            DICD_GENERATE_ID,
            &device)) {
        ThrowLastError("SetupDiCreateDeviceInfoW");
    }

    SetExactHardwareId(set.get(), &device);
    if (!SetupDiCallClassInstaller(DIF_REGISTERDEVICE, set.get(), &device)) {
        ThrowLastError("SetupDiCallClassInstaller(DIF_REGISTERDEVICE)");
    }
    const bool rebootRequired = DeviceInstallNeedsRestart(set.get(), &device);
    return {GetDeviceInstanceId(set.get(), &device), rebootRequired};
}

struct RemoveResult {
    bool removed = false;
    bool rebootRequired = false;
};

RemoveResult RemoveExactDevice(const std::wstring& instanceId) {
    DeviceInfoSet set(SetupDiCreateDeviceInfoList(nullptr, nullptr));
    SP_DEVINFO_DATA device{};
    device.cbSize = sizeof(device);
    if (!SetupDiOpenDeviceInfoW(set.get(), instanceId.c_str(), nullptr, 0, &device)) {
        const DWORD error = GetLastError();
        if (error == ERROR_NO_SUCH_DEVINST || error == ERROR_NO_SUCH_DEVICE || error == ERROR_NOT_FOUND) {
            return {};
        }
        ThrowLastError("SetupDiOpenDeviceInfoW");
    }

    if (!HasExactHardwareId(set.get(), &device)) {
        throw std::runtime_error(
            "recorded device instance no longer has hardware ID Root\\VoxveilVirtualAudio");
    }

    SP_REMOVEDEVICE_PARAMS remove{};
    remove.ClassInstallHeader.cbSize = sizeof(SP_CLASSINSTALL_HEADER);
    remove.ClassInstallHeader.InstallFunction = DIF_REMOVE;
    remove.Scope = DI_REMOVEDEVICE_GLOBAL;
    remove.HwProfile = 0;
    if (!SetupDiSetClassInstallParamsW(
            set.get(),
            &device,
            &remove.ClassInstallHeader,
            sizeof(remove))) {
        ThrowLastError("SetupDiSetClassInstallParamsW(DIF_REMOVE)");
    }
    if (!SetupDiCallClassInstaller(DIF_REMOVE, set.get(), &device)) {
        ThrowLastError("SetupDiCallClassInstaller(DIF_REMOVE)");
    }
    return {true, DeviceInstallNeedsRestart(set.get(), &device)};
}

bool ExactDeviceExists(const std::wstring& instanceId) {
    DeviceInfoSet set(SetupDiCreateDeviceInfoList(nullptr, nullptr));
    SP_DEVINFO_DATA device{};
    device.cbSize = sizeof(device);
    if (!SetupDiOpenDeviceInfoW(set.get(), instanceId.c_str(), nullptr, 0, &device)) {
        const DWORD error = GetLastError();
        if (error == ERROR_NO_SUCH_DEVINST || error == ERROR_NO_SUCH_DEVICE || error == ERROR_NOT_FOUND) {
            return false;
        }
        ThrowLastError("SetupDiOpenDeviceInfoW");
    }

    if (!HasExactHardwareId(set.get(), &device)) {
        throw std::runtime_error(
            "recorded device instance no longer has hardware ID Root\\VoxveilVirtualAudio");
    }
    return true;
}

int EnsureCommand(const std::wstring& infPath) {
    auto matches = FindMatchingDeviceInstanceIds();
    if (matches.size() > 1) {
        throw std::runtime_error(
            "more than one Root\\VoxveilVirtualAudio devnode exists; refusing to create another");
    }

    bool created = false;
    bool rebootRequired = false;
    std::wstring instanceId;
    if (matches.empty()) {
        const CreateResult result = CreateRootDevice(infPath);
        instanceId = result.instanceId;
        rebootRequired = result.rebootRequired;
        created = true;

        matches = FindMatchingDeviceInstanceIds();
        if (matches.size() != 1 || _wcsicmp(matches.front().c_str(), instanceId.c_str()) != 0) {
            try {
                RemoveExactDevice(instanceId);
            } catch (...) {
            }
            throw std::runtime_error(
                "registered root devnode did not re-resolve to exactly one Voxveil hardware ID");
        }
    } else {
        instanceId = matches.front();
    }

    std::wcout << L"instanceId=" << instanceId << L"\n";
    std::wcout << L"created=" << (created ? 1 : 0) << L"\n";
    std::wcout << L"rebootRequired=" << (rebootRequired ? 1 : 0) << L"\n";
    return 0;
}

int RemoveCommand(const std::wstring& instanceId) {
    if (instanceId.empty()) {
        throw std::runtime_error("device instance ID must not be empty");
    }
    const RemoveResult result = RemoveExactDevice(instanceId);
    std::wcout << L"removed=" << (result.removed ? 1 : 0) << L"\n";
    std::wcout << L"rebootRequired=" << (result.rebootRequired ? 1 : 0) << L"\n";
    return 0;
}

int QueryCommand(const std::wstring& instanceId) {
    if (instanceId.empty()) {
        throw std::runtime_error("device instance ID must not be empty");
    }
    const bool exists = ExactDeviceExists(instanceId);
    std::wcout << L"exists=" << (exists ? 1 : 0) << L"\n";
    return 0;
}

void PrintUsage() {
    std::wcerr << L"Usage:\n"
               << L"  voxveil-virtual-device.exe ensure <VoxveilVirtualAudio.inf>\n"
               << L"  voxveil-virtual-device.exe remove <device-instance-id>\n"
               << L"  voxveil-virtual-device.exe query <device-instance-id>\n";
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
    try {
        if (argc != 3) {
            PrintUsage();
            return 2;
        }

        const std::wstring command = argv[1];
        if (_wcsicmp(command.c_str(), L"ensure") == 0) {
            return EnsureCommand(argv[2]);
        }
        if (_wcsicmp(command.c_str(), L"remove") == 0) {
            return RemoveCommand(argv[2]);
        }
        if (_wcsicmp(command.c_str(), L"query") == 0) {
            return QueryCommand(argv[2]);
        }

        PrintUsage();
        return 2;
    } catch (const std::exception& error) {
        std::cerr << "voxveil-virtual-device: " << error.what() << '\n';
        return 1;
    }
}