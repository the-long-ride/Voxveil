#include <windows.h>
#include <wtypes.h>
#include <devpkey.h>
#include <propkeydef.h>
#include <functiondiscoverykeys_devpkey.h>
#include <mmdeviceapi.h>
#include <propvarutil.h>
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
constexpr wchar_t kCompositeEndpointEffectProperty[] =
    L"{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},15";
constexpr wchar_t kEndpointModesProperty[] = L"{D3993A3F-99C2-4402-B5EC-A92A0367664B},7";
constexpr wchar_t kAssociationProperty[] = L"{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},0";
constexpr wchar_t kVoxveilClsid[] = L"{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}";
constexpr wchar_t kDefaultMode[] = L"{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}";
constexpr wchar_t kRuntimeMarker[] = L"Voxveil.RuntimeRegistration";
constexpr wchar_t kAddedAssociationMarker[] = L"Voxveil.AddedAssociation";
constexpr wchar_t kAddedDefaultModeMarker[] = L"Voxveil.AddedDefaultMode";

struct InterfaceRecord {
    std::wstring instanceId;
    std::wstring hardwareId;
    std::wstring path;
    GUID containerId{};
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

std::wstring referenceFromPath(const std::wstring& path) {
    const size_t brace = path.find_last_of(L'}');
    if (brace == std::wstring::npos || brace + 1 >= path.size()) {
        return {};
    }
    size_t start = brace + 1;
    while (start < path.size() && (path[start] == L'\\' || path[start] == L'/')) {
        ++start;
    }
    return path.substr(start);
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
    return reinterpret_cast<const wchar_t*>(bytes.data());
}

GUID containerId(HDEVINFO info, SP_DEVINFO_DATA& device) {
    DEVPROPTYPE type = 0;
    GUID value{};
    DWORD required = 0;
    if (!SetupDiGetDevicePropertyW(info, &device, &DEVPKEY_Device_ContainerId, &type,
                                   reinterpret_cast<PBYTE>(&value), sizeof(value), &required, 0) ||
        type != DEVPROP_TYPE_GUID) {
        throw win32Error("SetupDiGetDevicePropertyW(DEVPKEY_Device_ContainerId)");
    }
    return value;
}

std::vector<InterfaceRecord> enumerateInterfaces(const GUID& category, bool hardware) {
    HDEVINFO info = SetupDiGetClassDevsW(&category, nullptr, nullptr,
                                         DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
    if (info == INVALID_HANDLE_VALUE) {
        throw win32Error("SetupDiGetClassDevsW");
    }
    std::vector<InterfaceRecord> records;
    try {
        for (DWORD index = 0;; ++index) {
            SP_DEVICE_INTERFACE_DATA iface{};
            iface.cbSize = sizeof(iface);
            if (!SetupDiEnumDeviceInterfaces(info, nullptr, &category, index, &iface)) {
                if (GetLastError() == ERROR_NO_MORE_ITEMS) break;
                throw win32Error("SetupDiEnumDeviceInterfaces");
            }
            DWORD required = 0;
            SetupDiGetDeviceInterfaceDetailW(info, &iface, nullptr, 0, &required, nullptr);
            if (GetLastError() != ERROR_INSUFFICIENT_BUFFER || required == 0) {
                throw win32Error("SetupDiGetDeviceInterfaceDetailW(size)");
            }
            std::vector<BYTE> detailBytes(required);
            auto* detail = reinterpret_cast<SP_DEVICE_INTERFACE_DETAIL_DATA_W*>(detailBytes.data());
            detail->cbSize = sizeof(SP_DEVICE_INTERFACE_DETAIL_DATA_W);
            SP_DEVINFO_DATA device{};
            device.cbSize = sizeof(device);
            if (!SetupDiGetDeviceInterfaceDetailW(info, &iface, detail, required, nullptr, &device)) {
                throw win32Error("SetupDiGetDeviceInterfaceDetailW(data)");
            }
            InterfaceRecord record;
            record.instanceId = instanceId(info, device);
            record.path = detail->DevicePath;
            record.containerId = containerId(info, device);
            if (hardware) record.hardwareId = firstHardwareId(info, device);
            records.push_back(std::move(record));
        }
    } catch (...) {
        SetupDiDestroyDeviceInfoList(info);
        throw;
    }
    SetupDiDestroyDeviceInfoList(info);
    return records;
}

GUID defaultRenderContainer() {
    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    const bool uninitialize = SUCCEEDED(init);
    if (FAILED(init) && init != RPC_E_CHANGED_MODE) {
        throw std::runtime_error("CoInitializeEx failed");
    }
    IMMDeviceEnumerator* enumerator = nullptr;
    IMMDevice* endpoint = nullptr;
    IPropertyStore* store = nullptr;
    PROPVARIANT value{};
    PropVariantInit(&value);
    GUID result{};
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  IID_PPV_ARGS(&enumerator));
    if (SUCCEEDED(hr)) hr = enumerator->GetDefaultAudioEndpoint(eRender, eMultimedia, &endpoint);
    if (SUCCEEDED(hr)) hr = endpoint->OpenPropertyStore(STGM_READ, &store);
    if (SUCCEEDED(hr)) hr = store->GetValue(PKEY_Device_ContainerId, &value);
    if (SUCCEEDED(hr) && value.vt == VT_CLSID && value.puuid != nullptr) {
        result = *value.puuid;
    } else if (SUCCEEDED(hr) && value.vt == VT_LPWSTR && value.pwszVal != nullptr) {
        hr = CLSIDFromString(value.pwszVal, &result);
    } else if (SUCCEEDED(hr)) {
        hr = E_UNEXPECTED;
    }
    PropVariantClear(&value);
    if (store) store->Release();
    if (endpoint) endpoint->Release();
    if (enumerator) enumerator->Release();
    if (uninitialize) CoUninitialize();
    if (FAILED(hr)) {
        throw std::runtime_error("Could not resolve the current default render endpoint container");
    }
    return result;
}

Target discoverDefaultTarget() {
    const GUID wanted = defaultRenderContainer();
    const auto renders = enumerateInterfaces(kCategoryRender, true);
    const auto topologies = enumerateInterfaces(kCategoryTopology, false);
    std::map<std::wstring, Target> candidates;
    for (const auto& render : renders) {
        if (!IsEqualGUID(render.containerId, wanted) || render.hardwareId.empty()) continue;
        const auto key = folded(render.instanceId);
        auto& target = candidates[key];
        target.instanceId = render.instanceId;
        target.hardwareId = render.hardwareId;
        std::set<std::wstring> seen;
        for (const auto& topology : topologies) {
            if (folded(topology.instanceId) != key) continue;
            const auto reference = referenceFromPath(topology.path);
            if (!reference.empty() && seen.insert(folded(reference)).second) {
                target.topologyRefs.push_back(reference);
            }
        }
    }
    std::erase_if(candidates, [](const auto& item) { return item.second.topologyRefs.empty(); });
    if (candidates.size() != 1) {
        throw std::runtime_error("Could not uniquely map the current default render endpoint to one PnP audio function device");
    }
    return std::move(candidates.begin()->second);
}

std::vector<std::wstring> readMultiSz(HKEY key, const wchar_t* name) {
    DWORD type = 0;
    DWORD bytes = 0;
    LONG status = RegQueryValueExW(key, name, nullptr, &type, nullptr, &bytes);
    if (status == ERROR_FILE_NOT_FOUND) return {};
    if (status != ERROR_SUCCESS || type != REG_MULTI_SZ) return {};
    std::vector<wchar_t> data(bytes / sizeof(wchar_t) + 2, L'\0');
    status = RegQueryValueExW(key, name, nullptr, &type,
                              reinterpret_cast<BYTE*>(data.data()), &bytes);
    if (status != ERROR_SUCCESS) throw registryError("RegQueryValueExW", status);
    std::vector<std::wstring> values;
    for (const wchar_t* cursor = data.data(); *cursor != L'\0';) {
        values.emplace_back(cursor);
        cursor += values.back().size() + 1;
    }
    return values;
}

void writeMultiSz(HKEY key, const wchar_t* name, const std::vector<std::wstring>& values) {
    if (values.empty()) {
        RegDeleteValueW(key, name);
        return;
    }
    std::vector<wchar_t> data;
    for (const auto& value : values) {
        data.insert(data.end(), value.begin(), value.end());
        data.push_back(L'\0');
    }
    data.push_back(L'\0');
    const LONG status = RegSetValueExW(key, name, 0, REG_MULTI_SZ,
                                       reinterpret_cast<const BYTE*>(data.data()),
                                       static_cast<DWORD>(data.size() * sizeof(wchar_t)));
    if (status != ERROR_SUCCESS) throw registryError("RegSetValueExW", status);
}

bool marker(HKEY key, const wchar_t* name) {
    DWORD value = 0, type = 0, size = sizeof(value);
    return RegQueryValueExW(key, name, nullptr, &type, reinterpret_cast<BYTE*>(&value), &size) ==
               ERROR_SUCCESS &&
           type == REG_DWORD && value == 1;
}

size_t cleanupLegacyRuntime() {
    HDEVINFO info = SetupDiGetClassDevsW(&kCategoryAudio, nullptr, nullptr,
                                         DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
    if (info == INVALID_HANDLE_VALUE) throw win32Error("SetupDiGetClassDevsW(KSCATEGORY_AUDIO)");
    size_t cleaned = 0;
    for (DWORD index = 0;; ++index) {
        SP_DEVICE_INTERFACE_DATA iface{};
        iface.cbSize = sizeof(iface);
        if (!SetupDiEnumDeviceInterfaces(info, nullptr, &kCategoryAudio, index, &iface)) {
            if (GetLastError() == ERROR_NO_MORE_ITEMS) break;
            SetupDiDestroyDeviceInfoList(info);
            throw win32Error("SetupDiEnumDeviceInterfaces(KSCATEGORY_AUDIO)");
        }
        HKEY interfaceKey = SetupDiOpenDeviceInterfaceRegKey(info, &iface, 0, KEY_READ | KEY_WRITE);
        if (interfaceKey == INVALID_HANDLE_VALUE) continue;
        HKEY fx = nullptr;
        if (RegOpenKeyExW(interfaceKey, kFxStore, 0, KEY_READ | KEY_WRITE, &fx) == ERROR_SUCCESS) {
            if (marker(fx, kRuntimeMarker)) {
                auto effects = readMultiSz(fx, kCompositeEndpointEffectProperty);
                std::erase_if(effects, [](const auto& value) { return folded(value) == folded(kVoxveilClsid); });
                writeMultiSz(fx, kCompositeEndpointEffectProperty, effects);
                if (marker(fx, kAddedDefaultModeMarker)) {
                    auto modes = readMultiSz(fx, kEndpointModesProperty);
                    std::erase_if(modes, [](const auto& value) { return folded(value) == folded(kDefaultMode); });
                    writeMultiSz(fx, kEndpointModesProperty, modes);
                }
                if (marker(fx, kAddedAssociationMarker) && effects.empty()) RegDeleteValueW(fx, kAssociationProperty);
                RegDeleteValueW(fx, kRuntimeMarker);
                RegDeleteValueW(fx, kAddedAssociationMarker);
                RegDeleteValueW(fx, kAddedDefaultModeMarker);
                ++cleaned;
            }
            RegCloseKey(fx);
        }
        RegCloseKey(interfaceKey);
    }
    SetupDiDestroyDeviceInfoList(info);
    return cleaned;
}

std::string utf8(const std::wstring& value) {
    if (value.empty()) return {};
    const int size = WideCharToMultiByte(CP_UTF8, 0, value.data(), static_cast<int>(value.size()),
                                         nullptr, 0, nullptr, nullptr);
    std::string out(static_cast<size_t>(size), '\0');
    WideCharToMultiByte(CP_UTF8, 0, value.data(), static_cast<int>(value.size()), out.data(), size,
                        nullptr, nullptr);
    return out;
}

std::string jsonEscape(const std::wstring& value) {
    std::string out;
    for (char ch : utf8(value)) {
        if (ch == '\\' || ch == '"') out.push_back('\\');
        out.push_back(ch);
    }
    return out;
}

void printTarget(const Target& target) {
    std::cout << "[{\"instanceId\":\"" << jsonEscape(target.instanceId)
              << "\",\"hardwareId\":\"" << jsonEscape(target.hardwareId)
              << "\",\"topologyRefs\":[";
    for (size_t i = 0; i < target.topologyRefs.size(); ++i) {
        if (i) std::cout << ',';
        std::cout << '"' << jsonEscape(target.topologyRefs[i]) << '"';
    }
    std::cout << "]}]\n";
}

bool selfTest() {
    return referenceFromPath(LR"(\\?\USB#A#{DDA54A40-1E4C-11D1-A050-405705C10000}\Speaker1)") ==
               L"Speaker1" &&
           folded(L"Render") == L"render";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    try {
        if (argc == 2 && std::wstring(argv[1]) == L"--self-test") {
            if (!selfTest()) return 2;
            std::cout << "VoxveilApoTarget self-test passed\n";
            return 0;
        }
        if (argc == 2 && std::wstring(argv[1]) == L"--cleanup-runtime") {
            std::cout << "Removed legacy Voxveil runtime FX registration from "
                      << cleanupLegacyRuntime() << " interface(s).\n";
            return 0;
        }
        printTarget(discoverDefaultTarget());
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "Voxveil audio target operation failed: " << error.what() << '\n';
        return 1;
    }
}
