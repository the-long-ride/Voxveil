#include "voxveil_apo.h"

#include <atomic>
#include <new>

namespace {

class VoxveilClassFactory final : public IClassFactory {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) override {
        if (object == nullptr) {
            return E_POINTER;
        }
        *object = nullptr;
        if (iid == IID_IUnknown || iid == IID_IClassFactory) {
            *object = static_cast<IClassFactory*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    ULONG STDMETHODCALLTYPE AddRef() override {
        return refs_.fetch_add(1, std::memory_order_relaxed) + 1;
    }

    ULONG STDMETHODCALLTYPE Release() override {
        const ULONG remaining = refs_.fetch_sub(1, std::memory_order_acq_rel) - 1;
        if (remaining == 0) {
            delete this;
        }
        return remaining;
    }

    HRESULT STDMETHODCALLTYPE CreateInstance(
        IUnknown* outer,
        REFIID iid,
        void** object) override {
        if (object == nullptr) {
            return E_POINTER;
        }
        *object = nullptr;
        if (outer != nullptr) {
            return CLASS_E_NOAGGREGATION;
        }

        CComObject<CVoxveilApo>* instance = nullptr;
        HRESULT result = CComObject<CVoxveilApo>::CreateInstance(&instance);
        if (FAILED(result) || instance == nullptr) {
            return FAILED(result) ? result : E_OUTOFMEMORY;
        }
        instance->AddRef();
        result = instance->QueryInterface(iid, object);
        instance->Release();
        return result;
    }

    HRESULT STDMETHODCALLTYPE LockServer(BOOL) override {
        return S_OK;
    }

private:
    std::atomic<ULONG> refs_{1};
};

} // namespace

extern "C" BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        DisableThreadLibraryCalls(instance);
    }
    return TRUE;
}

extern "C" HRESULT __stdcall DllGetClassObject(
    REFCLSID clsid,
    REFIID iid,
    void** object) {
    if (!IsEqualCLSID(clsid, CLSID_VoxveilApo)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    auto* factory = new (std::nothrow) VoxveilClassFactory();
    if (factory == nullptr) {
        return E_OUTOFMEMORY;
    }
    const HRESULT result = factory->QueryInterface(iid, object);
    factory->Release();
    return result;
}

extern "C" HRESULT __stdcall DllCanUnloadNow() {
    // AudioDG owns the lifetime of system-effect APOs. Keep the module loaded
    // for the process lifetime so no realtime callback can race an unload.
    return S_FALSE;
}
