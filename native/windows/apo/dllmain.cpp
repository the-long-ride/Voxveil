#include <atlbase.h>
#include <atlcom.h>

#include "VoxveilApo.h"

class CVoxveilApoModule final : public ATL::CAtlDllModuleT<CVoxveilApoModule> {};
CVoxveilApoModule _AtlModule;

extern "C" BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    return _AtlModule.DllMain(reason, reserved);
}

extern "C" STDAPI DllCanUnloadNow(void) {
    return _AtlModule.DllCanUnloadNow();
}

extern "C" STDAPI DllGetClassObject(REFCLSID clsid, REFIID iid, LPVOID* object) {
    return _AtlModule.DllGetClassObject(clsid, iid, object);
}
