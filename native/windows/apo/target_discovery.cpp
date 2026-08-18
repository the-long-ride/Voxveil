#include <windows.h>
#include <setupapi.h>

#include <algorithm>
#include <cwctype>
#include <iostream>
#include <map>
#include <set>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace {

constexpr GUID kCategoryAudio = {
    0x6994ad04, 0x93ef, 0x11d0, {0xa3, 0xcc, 0x00, 0xa0, 0xc9, 0x22, 0x31, 0x96}};
constexpr GUID kCategoryRender = {
    0x65e8773e, 0x8f56, 0x11d0, {0xa3, 0xb9, 0x00, 0xa0, 0xc9, 0x22, 0x31, 0x96}};
constexpr GUID kCategoryTopology = {
    0xdda54a40, 0x1e4c, 0x11d1, {0xa0, 0x50, 0x40, 0x57, 0x05, 0xc1, 0x00, 0x00}};

constexpr wchar_t kFxStore[] = L"FX\\0";
constexpr wchar_t kAssociationProperty[] = L"{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},0";
constexpr wchar_t kCompositeEndpointEffectProperty[] =
    L"{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},15";
constexpr wchar_t kEndpointModesProperty[] = L"{D3993A3F-99C2-4402-B5EC-A92A0367664B},7";
constexpr wchar_t kAnyNode[] = L"{00000000-0000-0000-0000-000000000000}";
constexpr wchar_t kVoxveilClsid[] = L"{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}";
constexpr wchar_t kDefaultMode[] = L"{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}";
constexpr wchar_t kRuntimeMarker[] = L"Voxveil.RuntimeRegistration";
constexpr wchar_t kAddedAssociationMarker[] = L"Voxveil.AddedAssociation";
constexpr wchar_t kAddedDefaultModeMarker[] = L"Voxveil.AddedDefaultMode";

struct InterfaceRecord {
    std::wstring instanceId;
    std::wstring hardwareId;
    std::wstring path;
    std::wstring reference;
};

struct Target {
    std::wstring instanceId;
    std::wstring hardwareId;
    std::vector<std::wstring> topologyRefs;
};

std::runtime_error win32Error(const char* operation) {
    return std::runtime_error(std::string(operation) + " failed with Win32 error " +
                              std::to_string(GetLastError()));
}

std::runtime_error registryError(const char* operation, LONG status) {
    return std::runtime_error(std::string(operation) + " failed with registry error " +
                              std::to_string(status));
}

std::wstring folded(std::wstring value) {
    std::transform(value.begin(), value.end(), value.begin(), [](wchar_t ch) {
        return static_cast<wchar_t>(std::towlower(ch));
    });
    return value;
}

bool sameText(const std::wstring& left, const std::wstring& right) {
    return folded(left) == folded(right);
}

std::string utf8(const std::wstring& value) {
    if (value.empty()) {
        return {};
    }
    const int length = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                                           static_cast<int>(value.size()), nullptr, 0, nullptr,
                                           nullptr);
    if (length <= 0) {
        throw win32Error("WideCharToMultiByte(size)");
    }
    std::string result(static_cast<size_t>(length), '\0');
    if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                            static_cast<int>(value.size()), result.data(), length, nullptr,
                            nullptr) != length) {
        throw win32Error("WideCharToMultiByte(data)");
    }
    return result;
}

std::string jsonEscape(const std::wstring& value) {
    const std::string input = utf8(value);
    std::string out;
    out.reserve(input.size() + 8);
    for (const unsigned char ch : input) {
        switch (ch) {
        case '\\': out += "\\\\"; break;
        case '"': out += "\\\""; break;
        case '\b': out += "\\b"; break;
        case '\f': out += "\\f"; break;
        case '\n': out += "\\n"; break;
        case '\r': out += "\\r"; break;
        case '\t': out += "\\t"; break;
        default:
            if (ch < 0x20) {
                static constexpr char hex[] = "0123456789abcdef";
                out += "\\u00";
                out += hex[(ch >> 4) & 0x0f];
                out += hex[ch & 0x0f];
            } else {
                out += static_cast<char>(ch);
            }
        }
    }
    return out;
}

std::wstring instanceId(HDEVINFO info, SP_DEVINFO_DATA& device) {
    DWORD required = 0;
    SetupDiGetDeviceInstanceIdW(info, &device, nullptr, 0, &required);
    if (GetLastError() != ERROR_INSUFFICIENT_BUFFER || required == 0) {
        throw win32Error("SetupDiGetDeviceInstanceIdW(size)");
    }
    std::vector<wchar_t> buffer(required);
    if (!SetupDiGetDeviceInstanceIdW(info, &device, buffer.data(), required, nullptr)) {
        throw win32Error("SetupDiGetDeviceInstanceIdW(data)");
    }
    return buffer.data();
}

std::wstring firstHardwareId(HDEVINFO info, SP_DEVINFO_DATA& device) {
    DWORD type = 0;
    DWORD required = 0;
    SetupDiGetDeviceRegistryPropertyW(info, &device, SPDRP_HARDWAREID, &type, nullptr, 0,
                                      &required);
    if (GetLastError() != ERROR_INSUFFICIENT_BUFFER || required < sizeof(wchar_t)) {
        return {};
    }
    std::vector<BYTE> bytes(required + sizeof(wchar_t), 0);
    if (!SetupDiGetDeviceRegistryPropertyW(info, &device, SPDRP_HARDWAREID, &type, bytes.data(),
                                           required, nullptr)) {
        throw win32Error("SetupDiGetDeviceRegistryPropertyW(SPDRP_HARDWAREID)");
    }
    if (type != REG_MULTI_SZ && type != REG_SZ) {
        return {};
    }
    return reinterpret_cast<const wchar_t*>(bytes.data());
}

std::wstring referenceFromPath(const std::wstring& path) {
    const size_t closingBrace = path.find_last_of(L'}');
    if (closingBrace == std::wstring::npos || closingBrace + 1 >= path.size()) {
        return {};
    }
    size_t start = closingBrace + 1;
    while (start < path.size() && (path[start] == L'\\' || path[start] == L'/')) {
        ++start;
    }
    return path.substr(start);
}

std::vector<InterfaceRecord> enumerateInterfaces(const GUID& interfaceClass,
                                                  bool includeHardwareId) {
    HDEVINFO info = SetupDiGetClassDevsW(&interfaceClass, nullptr, nullptr,
                                         DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
    if (info == INVALID_HANDLE_VALUE) {
        throw win32Error("SetupDiGetClassDevsW");
    }
    std::vector<InterfaceRecord> records;
    try {
        for (DWORD index = 0;; ++index) {
            SP_DEVICE_INTERFACE_DATA interfaceData{};
            interfaceData.cbSize = sizeof(interfaceData);
            if (!SetupDiEnumDeviceInterfaces(info, nullptr, &interfaceClass, index,
                                             &interfaceData)) {
                if (GetLastError() == ERROR_NO_MORE_ITEMS) {
                    break;
                }
                throw win32Error("SetupDiEnumDeviceInterfaces");
            }
            DWORD required = 0;
            SetupDiGetDeviceInterfaceDetailW(info, &interfaceData, nullptr, 0, &required, nullptr);
            if (GetLastError() != ERROR_INSUFFICIENT_BUFFER || required == 0) {
                throw win32Error("SetupDiGetDeviceInterfaceDetailW(size)");
            }
            std::vector<BYTE> detailBytes(required);
            auto* detail = reinterpret_cast<SP_DEVICE_INTERFACE_DETAIL_DATA_W*>(detailBytes.data());
            detail->cbSize = sizeof(SP_DEVICE_INTERFACE_DETAIL_DATA_W);
            SP_DEVINFO_DATA device{};
            device.cbSize = sizeof(device);
            if (!SetupDiGetDeviceInterfaceDetailW(info, &interfaceData, detail, required, nullptr,
                                                  &device)) {
                throw win32Error("SetupDiGetDeviceInterfaceDetailW(data)");
            }
            InterfaceRecord record;
            record.instanceId = instanceId(info, device);
            record.path = detail->DevicePath;
            record.reference = referenceFromPath(record.path);
            if (includeHardwareId) {
                record.hardwareId = firstHardwareId(info, device);
            }
            records.push_back(std::move(record));
        }
    } catch (...) {
        SetupDiDestroyDeviceInfoList(info);
        throw;
    }
    SetupDiDestroyDeviceInfoList(info);
    return records;
}

std::vector<std::wstring> readMultiSz(HKEY key, const wchar_t* name) {
    DWORD type = 0;
    DWORD size = 0;
    LONG status = RegQueryValueExW(key, name, nullptr, &type, nullptr, &size);
    if (status == ERROR_FILE_NOT_FOUND) {
        return {};
    }
    if (status != ERROR_SUCCESS) {
        throw registryError("RegQueryValueExW(size)", status);
    }
    if (type != REG_MULTI_SZ) {
        throw std::runtime_error("existing audio FX property is not REG_MULTI_SZ");
    }
    std::vector<wchar_t> data(size / sizeof(wchar_t) + 2, L'\0');
    status = RegQueryValueExW(key, name, nullptr, &type,
                              reinterpret_cast<BYTE*>(data.data()), &size);
    if (status != ERROR_SUCCESS) {
        throw registryError("RegQueryValueExW(data)", status);
    }
    std::vector<std::wstring> values;
    for (const wchar_t* cursor = data.data(); *cursor != L'\0';) {
        std::wstring value(cursor);
        values.push_back(value);
        cursor += value.size() + 1;
    }
    return values;
}

void writeMultiSz(HKEY key, const wchar_t* name, const std::vector<std::wstring>& values) {
    if (values.empty()) {
        const LONG status = RegDeleteValueW(key, name);
        if (status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND) {
            throw registryError("RegDeleteValueW", status);
        }
        return;
    }
    std::vector<wchar_t> data;
    for (const auto& value : values) {
        data.insert(data.end(), value.begin(), value.end());
        data.push_back(L'\0');
    }
    data.push_back(L'\0');
    const LONG status = RegSetValueExW(
        key, name, 0, REG_MULTI_SZ, reinterpret_cast<const BYTE*>(data.data()),
        static_cast<DWORD>(data.size() * sizeof(wchar_t)));
    if (status != ERROR_SUCCESS) {
        throw registryError("RegSetValueExW(REG_MULTI_SZ)", status);
    }
}

bool appendUnique(std::vector<std::wstring>& values, const std::wstring& value) {
    if (std::any_of(values.begin(), values.end(), [&](const auto& existing) {
            return sameText(existing, value);
        })) {
        return false;
    }
    values.push_back(value);
    return true;
}

bool removeValue(std::vector<std::wstring>& values, const std::wstring& value) {
    const auto before = values.size();
    std::erase_if(values, [&](const auto& existing) { return sameText(existing, value); });
    return values.size() != before;
}

bool hasValue(HKEY key, const wchar_t* name) {
    const LONG status = RegQueryValueExW(key, name, nullptr, nullptr, nullptr, nullptr);
    if (status == ERROR_FILE_NOT_FOUND) {
        return false;
    }
    if (status != ERROR_SUCCESS) {
        throw registryError("RegQueryValueExW(exists)", status);
    }
    return true;
}

bool markerSet(HKEY key, const wchar_t* name) {
    DWORD value = 0;
    DWORD type = 0;
    DWORD size = sizeof(value);
    const LONG status = RegQueryValueExW(key, name, nullptr, &type,
                                         reinterpret_cast<BYTE*>(&value), &size);
    return status == ERROR_SUCCESS && type == REG_DWORD && value == 1;
}

void setDword(HKEY key, const wchar_t* name, DWORD value) {
    const LONG status = RegSetValueExW(key, name, 0, REG_DWORD,
                                       reinterpret_cast<const BYTE*>(&value), sizeof(value));
    if (status != ERROR_SUCCESS) {
        throw registryError("RegSetValueExW(REG_DWORD)", status);
    }
}

void setString(HKEY key, const wchar_t* name, const wchar_t* value) {
    const DWORD size = static_cast<DWORD>((std::wcslen(value) + 1) * sizeof(wchar_t));
    const LONG status = RegSetValueExW(key, name, 0, REG_SZ,
                                       reinterpret_cast<const BYTE*>(value), size);
    if (status != ERROR_SUCCESS) {
        throw registryError("RegSetValueExW(REG_SZ)", status);
    }
}

bool installFx(HKEY interfaceKey) {
    HKEY fx = nullptr;
    const LONG createStatus = RegCreateKeyExW(interfaceKey, kFxStore, 0, nullptr, 0,
                                               KEY_READ | KEY_WRITE, nullptr, &fx, nullptr);
    if (createStatus != ERROR_SUCCESS) {
        throw registryError("RegCreateKeyExW(FX\\0)", createStatus);
    }
    try {
        if (!hasValue(fx, kAssociationProperty)) {
            setString(fx, kAssociationProperty, kAnyNode);
            setDword(fx, kAddedAssociationMarker, 1);
        }
        auto effects = readMultiSz(fx, kCompositeEndpointEffectProperty);
        if (appendUnique(effects, kVoxveilClsid)) {
            writeMultiSz(fx, kCompositeEndpointEffectProperty, effects);
        }
        auto modes = readMultiSz(fx, kEndpointModesProperty);
        if (appendUnique(modes, kDefaultMode)) {
            writeMultiSz(fx, kEndpointModesProperty, modes);
            setDword(fx, kAddedDefaultModeMarker, 1);
        }
        setDword(fx, kRuntimeMarker, 1);
        RegCloseKey(fx);
        return true;
    } catch (...) {
        RegCloseKey(fx);
        throw;
    }
}

bool removeFx(HKEY interfaceKey) {
    HKEY fx = nullptr;
    const LONG openStatus = RegOpenKeyExW(interfaceKey, kFxStore, 0, KEY_READ | KEY_WRITE, &fx);
    if (openStatus == ERROR_FILE_NOT_FOUND) {
        return false;
    }
    if (openStatus != ERROR_SUCCESS) {
        throw registryError("RegOpenKeyExW(FX\\0)", openStatus);
    }
    try {
        if (!markerSet(fx, kRuntimeMarker)) {
            RegCloseKey(fx);
            return false;
        }
        auto effects = readMultiSz(fx, kCompositeEndpointEffectProperty);
        if (removeValue(effects, kVoxveilClsid)) {
            writeMultiSz(fx, kCompositeEndpointEffectProperty, effects);
        }
        if (markerSet(fx, kAddedDefaultModeMarker)) {
            auto modes = readMultiSz(fx, kEndpointModesProperty);
            if (removeValue(modes, kDefaultMode)) {
                writeMultiSz(fx, kEndpointModesProperty, modes);
            }
        }
        if (markerSet(fx, kAddedAssociationMarker) && effects.empty()) {
            RegDeleteValueW(fx, kAssociationProperty);
        }
        RegDeleteValueW(fx, kRuntimeMarker);
        RegDeleteValueW(fx, kAddedAssociationMarker);
        RegDeleteValueW(fx, kAddedDefaultModeMarker);
        RegCloseKey(fx);
        return true;
    } catch (...) {
        RegCloseKey(fx);
        throw;
    }
}

size_t mutateTopologyAudioInterfaces(bool install) {
    const auto topologies = enumerateInterfaces(kCategoryTopology, false);
    std::set<std::pair<std::wstring, std::wstring>> topologyKeys;
    for (const auto& topology : topologies) {
        topologyKeys.emplace(folded(topology.instanceId), folded(topology.reference));
    }

    HDEVINFO info = SetupDiGetClassDevsW(&kCategoryAudio, nullptr, nullptr,
                                         DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
    if (info == INVALID_HANDLE_VALUE) {
        throw win32Error("SetupDiGetClassDevsW(KSCATEGORY_AUDIO)");
    }
    size_t changed = 0;
    try {
        for (DWORD index = 0;; ++index) {
            SP_DEVICE_INTERFACE_DATA interfaceData{};
            interfaceData.cbSize = sizeof(interfaceData);
            if (!SetupDiEnumDeviceInterfaces(info, nullptr, &kCategoryAudio, index, &interfaceData)) {
                if (GetLastError() == ERROR_NO_MORE_ITEMS) {
                    break;
                }
                throw win32Error("SetupDiEnumDeviceInterfaces(KSCATEGORY_AUDIO)");
            }
            DWORD required = 0;
            SetupDiGetDeviceInterfaceDetailW(info, &interfaceData, nullptr, 0, &required, nullptr);
            if (GetLastError() != ERROR_INSUFFICIENT_BUFFER || required == 0) {
                throw win32Error("SetupDiGetDeviceInterfaceDetailW(KSCATEGORY_AUDIO size)");
            }
            std::vector<BYTE> detailBytes(required);
            auto* detail = reinterpret_cast<SP_DEVICE_INTERFACE_DETAIL_DATA_W*>(detailBytes.data());
            detail->cbSize = sizeof(SP_DEVICE_INTERFACE_DETAIL_DATA_W);
            SP_DEVINFO_DATA device{};
            device.cbSize = sizeof(device);
            if (!SetupDiGetDeviceInterfaceDetailW(info, &interfaceData, detail, required, nullptr,
                                                  &device)) {
                throw win32Error("SetupDiGetDeviceInterfaceDetailW(KSCATEGORY_AUDIO data)");
            }
            const auto identity = std::make_pair(
                folded(instanceId(info, device)), folded(referenceFromPath(detail->DevicePath)));
            if (!topologyKeys.contains(identity)) {
                continue;
            }

            HKEY interfaceKey = install
                ? SetupDiCreateDeviceInterfaceRegKeyW(info, &interfaceData, 0,
                                                      KEY_READ | KEY_WRITE, nullptr, nullptr)
                : SetupDiOpenDeviceInterfaceRegKeyW(info, &interfaceData, 0,
                                                    KEY_READ | KEY_WRITE);
            if (interfaceKey == INVALID_HANDLE_VALUE) {
                if (!install && GetLastError() == ERROR_FILE_NOT_FOUND) {
                    continue;
                }
                throw win32Error(install ? "SetupDiCreateDeviceInterfaceRegKeyW"
                                         : "SetupDiOpenDeviceInterfaceRegKeyW");
            }
            const bool didChange = install ? installFx(interfaceKey) : removeFx(interfaceKey);
            RegCloseKey(interfaceKey);
            if (didChange) {
                ++changed;
            }
        }
    } catch (...) {
        SetupDiDestroyDeviceInfoList(info);
        throw;
    }
    SetupDiDestroyDeviceInfoList(info);
    if (install && changed == 0) {
        throw std::runtime_error("No KSCATEGORY_AUDIO topology interfaces were available for Voxveil FX registration");
    }
    return changed;
}

std::vector<Target> discoverTargets() {
    const auto renders = enumerateInterfaces(kCategoryRender, true);
    const auto topologies = enumerateInterfaces(kCategoryTopology, false);
    std::multimap<std::wstring, std::wstring> topologyByInstance;
    for (const auto& topology : topologies) {
        topologyByInstance.emplace(folded(topology.instanceId), topology.reference);
    }
    std::map<std::wstring, Target> targets;
    for (const auto& render : renders) {
        if (render.hardwareId.empty()) {
            continue;
        }
        const std::wstring key = folded(render.instanceId);
        const auto range = topologyByInstance.equal_range(key);
        if (range.first == range.second) {
            continue;
        }
        auto [it, inserted] = targets.try_emplace(key);
        Target& target = it->second;
        if (inserted) {
            target.instanceId = render.instanceId;
            target.hardwareId = render.hardwareId;
        }
        std::set<std::wstring> seen(target.topologyRefs.begin(), target.topologyRefs.end());
        for (auto topology = range.first; topology != range.second; ++topology) {
            if (seen.insert(topology->second).second) {
                target.topologyRefs.push_back(topology->second);
            }
        }
    }
    std::vector<Target> result;
    for (auto& [_, target] : targets) {
        result.push_back(std::move(target));
    }
    return result;
}

void printJson(const std::vector<Target>& targets) {
    std::cout << '[';
    for (size_t i = 0; i < targets.size(); ++i) {
        if (i != 0) {
            std::cout << ',';
        }
        const auto& target = targets[i];
        std::cout << "{\"instanceId\":\"" << jsonEscape(target.instanceId)
                  << "\",\"hardwareId\":\"" << jsonEscape(target.hardwareId)
                  << "\",\"topologyRefs\":[";
        for (size_t r = 0; r < target.topologyRefs.size(); ++r) {
            if (r != 0) {
                std::cout << ',';
            }
            std::cout << '"' << jsonEscape(target.topologyRefs[r]) << '"';
        }
        std::cout << "]}";
    }
    std::cout << "]\n";
}

bool selfTest() {
    const std::wstring path =
        LR"(\\?\HDAUDIO#FUNC_01&VEN_1234#A#{DDA54A40-1E4C-11D1-A050-405705C10000}\Topology)";
    std::vector<std::wstring> values{L"{11111111-1111-1111-1111-111111111111}"};
    const bool added = appendUnique(values, kVoxveilClsid);
    const bool duplicate = appendUnique(values, L"{7e268e67-2f3c-4f0a-a09c-8b7d27b43f51}");
    const bool removed = removeValue(values, kVoxveilClsid);
    return referenceFromPath(path) == L"Topology" && referenceFromPath(L"plain").empty() &&
           added && !duplicate && removed && values.size() == 1;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    try {
        if (argc == 2) {
            const std::wstring command(argv[1]);
            if (command == L"--self-test") {
                if (!selfTest()) {
                    std::cerr << "VoxveilApoTarget self-test failed\n";
                    return 2;
                }
                std::cout << "VoxveilApoTarget self-test passed\n";
                return 0;
            }
            if (command == L"--install-fx") {
                const size_t count = mutateTopologyAudioInterfaces(true);
                std::cout << "Registered Voxveil FX on " << count
                          << " KSCATEGORY_AUDIO topology interface(s).\n";
                return 0;
            }
            if (command == L"--remove-fx") {
                const size_t count = mutateTopologyAudioInterfaces(false);
                std::cout << "Removed Voxveil FX from " << count
                          << " KSCATEGORY_AUDIO topology interface(s).\n";
                return 0;
            }
        }

        const auto targets = discoverTargets();
        if (targets.empty()) {
            std::cerr << "No enabled Windows render device with a matching topology interface was found.\n";
            return 3;
        }
        printJson(targets);
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "Voxveil audio target operation failed: " << error.what() << '\n';
        return 1;
    }
}
