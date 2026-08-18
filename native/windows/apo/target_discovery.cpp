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

constexpr GUID kCategoryRender = {
    0x65e8773e, 0x8f56, 0x11d0, {0xa3, 0xb9, 0x00, 0xa0, 0xc9, 0x22, 0x31, 0x96}};
constexpr GUID kCategoryTopology = {
    0xdda54a40, 0x1e4c, 0x11d1, {0xa0, 0x50, 0x40, 0x57, 0x05, 0xc1, 0x00, 0x00}};

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

std::wstring folded(std::wstring value) {
    std::transform(value.begin(), value.end(), value.begin(), [](wchar_t ch) {
        return static_cast<wchar_t>(std::towlower(ch));
    });
    return value;
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
    result.reserve(targets.size());
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
    return referenceFromPath(path) == L"Topology" && referenceFromPath(L"plain").empty();
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    try {
        if (argc == 2 && std::wstring(argv[1]) == L"--self-test") {
            if (!selfTest()) {
                std::cerr << "VoxveilApoTarget self-test failed\n";
                return 2;
            }
            std::cout << "VoxveilApoTarget self-test passed\n";
            return 0;
        }

        const auto targets = discoverTargets();
        if (targets.empty()) {
            std::cerr << "No enabled Windows render device with a matching topology interface was found.\n";
            return 3;
        }
        printJson(targets);
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "Voxveil audio target discovery failed: " << error.what() << '\n';
        return 1;
    }
}
