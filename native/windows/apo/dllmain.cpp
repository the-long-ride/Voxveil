#include <atlbase.h>
#include <atlcom.h>

#include "VoxveilApo.h"

#define VOXVEIL_STRINGIZE_INNER(value) #value
#define VOXVEIL_STRINGIZE(value) VOXVEIL_STRINGIZE_INNER(value)

#ifndef VOXVEIL_BUILD_COMMIT
#define VOXVEIL_BUILD_COMMIT development
#endif

extern "C" __declspec(dllexport) const char* WINAPI VoxveilBuildCommitMarker() noexcept {
    static const char marker[] = "VOXVEIL_BUILD_COMMIT=" VOXVEIL_STRINGIZE(VOXVEIL_BUILD_COMMIT);
    return marker;
}

class CVoxveilApoModule final : public ATL::CAtlDllModuleT<CVoxveilApoModule> {};
CVoxveilApoModule _AtlModule;

extern "C" BOOL WINAPI DllMain(HINSTANCE, DWORD reason, LPVOID reserved) {
    return _AtlModule.DllMain(reason, reserved);
}

extern "C" STDAPI DllCanUnloadNow(void) {
    return _AtlModule.DllCanUnloadNow();
}

extern "C" STDAPI DllGetClassObject(REFCLSID clsid, REFIID iid, LPVOID* object) {
    return _AtlModule.DllGetClassObject(clsid, iid, object);
}
