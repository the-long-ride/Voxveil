#include <windows.h>
#include <audioenginebaseapo.h>

#include <iomanip>
#include <iostream>
#include <string>

namespace {

const CLSID kVoxveilApo = {
    0x7e268e67,
    0x2f3c,
    0x4f0a,
    {0xa0, 0x9c, 0x8b, 0x7d, 0x27, 0xb4, 0x3f, 0x51}};

using DllGetClassObjectFn = HRESULT(__stdcall*)(REFCLSID, REFIID, void**);

int fail(const wchar_t* step, HRESULT result) {
    std::wcerr << L"Voxveil APO check failed at " << step << L": HRESULT 0x"
               << std::hex << std::uppercase << static_cast<unsigned long>(result) << L'\n';
    return 1;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2 || argv[1] == nullptr || argv[1][0] == L'\0') {
        std::wcerr << L"Usage: VoxveilApoCheck.exe <path-to-VoxveilApo.dll>\n";
        return 2;
    }

    const HRESULT com_result = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    const bool uninitialize = SUCCEEDED(com_result);
    if (FAILED(com_result) && com_result != RPC_E_CHANGED_MODE) {
        return fail(L"CoInitializeEx", com_result);
    }

    HMODULE module = LoadLibraryW(argv[1]);
    if (module == nullptr) {
        const DWORD error = GetLastError();
        std::wcerr << L"Voxveil APO check could not load DLL (Win32 " << error << L"): "
                   << argv[1] << L'\n';
        if (uninitialize) {
            CoUninitialize();
        }
        return 3;
    }

    const auto get_class_object = reinterpret_cast<DllGetClassObjectFn>(
        GetProcAddress(module, "DllGetClassObject"));
    if (get_class_object == nullptr) {
        std::wcerr << L"Voxveil APO check could not find DllGetClassObject\n";
        FreeLibrary(module);
        if (uninitialize) {
            CoUninitialize();
        }
        return 4;
    }

    IClassFactory* factory = nullptr;
    HRESULT result = get_class_object(
        kVoxveilApo,
        IID_IClassFactory,
        reinterpret_cast<void**>(&factory));
    if (FAILED(result) || factory == nullptr) {
        FreeLibrary(module);
        if (uninitialize) {
            CoUninitialize();
        }
        return fail(L"DllGetClassObject", FAILED(result) ? result : E_POINTER);
    }

    IUnknown* unknown = nullptr;
    result = factory->CreateInstance(
        nullptr,
        IID_IUnknown,
        reinterpret_cast<void**>(&unknown));
    factory->Release();
    if (FAILED(result) || unknown == nullptr) {
        FreeLibrary(module);
        if (uninitialize) {
            CoUninitialize();
        }
        return fail(L"IClassFactory::CreateInstance", FAILED(result) ? result : E_POINTER);
    }

    IAudioProcessingObject* processing = nullptr;
    result = unknown->QueryInterface(
        __uuidof(IAudioProcessingObject),
        reinterpret_cast<void**>(&processing));
    if (FAILED(result) || processing == nullptr) {
        unknown->Release();
        FreeLibrary(module);
        if (uninitialize) {
            CoUninitialize();
        }
        return fail(L"IAudioProcessingObject QueryInterface", FAILED(result) ? result : E_POINTER);
    }

    IAudioProcessingObjectRT* realtime = nullptr;
    result = unknown->QueryInterface(
        __uuidof(IAudioProcessingObjectRT),
        reinterpret_cast<void**>(&realtime));
    if (FAILED(result) || realtime == nullptr) {
        processing->Release();
        unknown->Release();
        FreeLibrary(module);
        if (uninitialize) {
            CoUninitialize();
        }
        return fail(L"IAudioProcessingObjectRT QueryInterface", FAILED(result) ? result : E_POINTER);
    }

    PAPO_REG_PROPERTIES properties = nullptr;
    result = processing->GetRegistrationProperties(&properties);
    if (FAILED(result) || properties == nullptr || !IsEqualCLSID(properties->clsid, kVoxveilApo)) {
        if (properties != nullptr) {
            CoTaskMemFree(properties);
        }
        realtime->Release();
        processing->Release();
        unknown->Release();
        FreeLibrary(module);
        if (uninitialize) {
            CoUninitialize();
        }
        return fail(L"GetRegistrationProperties", FAILED(result) ? result : E_FAIL);
    }
    CoTaskMemFree(properties);

    realtime->Release();
    processing->Release();
    unknown->Release();
    FreeLibrary(module);
    if (uninitialize) {
        CoUninitialize();
    }

    std::wcout << L"Voxveil APO COM activation check passed\n";
    return 0;
}
