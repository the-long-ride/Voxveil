#include "voxveil_apo.h"

class VoxveilApoModule final : public CAtlDllModuleT<VoxveilApoModule> {};

VoxveilApoModule _AtlModule;

extern "C" BOOL WINAPI DllMain(HINSTANCE, DWORD reason, LPVOID reserved) {
    return _AtlModule.DllMain(reason, reserved);
}

extern "C" HRESULT __stdcall DllGetClassObject(
    REFCLSID clsid,
    REFIID iid,
    void** object) {
    return _AtlModule.DllGetClassObject(clsid, iid, object);
}

extern "C" HRESULT __stdcall DllCanUnloadNow() {
    return _AtlModule.DllCanUnloadNow();
}
