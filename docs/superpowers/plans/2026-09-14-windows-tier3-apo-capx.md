# Windows Tier 3 APO CAPX Modernization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernize the existing Voxveil componentized SFX APO for Windows 11 `IAudioSystemEffects3`/CAPX initialization and effect discovery while preserving Windows 10 compatibility and keeping production enablement gated on Microsoft/HLK validation.

**Architecture:** Extend the existing in-process APO rather than creating a second APO. The class implements `IAudioSystemEffects`, `IAudioSystemEffects2`, and `IAudioSystemEffects3`; accepts `APOInitSystemEffects`, `APOInitSystemEffects2`, or `APOInitSystemEffects3`; exposes one controllable Voxveil vocal-suppression effect; opens the CAPX effects property store when initialized with v3; and combines application enablement with the OS effect-state gate in the real-time processing path. Discovery-only Windows 11 instances must never increment the runtime `loadedInstances` readiness count.

**Tech Stack:** C++17, ATL/COM, Windows 11 `audioengineextensionapo.h`, `IAudioSystemEffectsPropertyStore`, WDK/MSBuild, INF/Extension INF, PowerShell, existing Rust APO readiness probe.

**Spec:** `docs/superpowers/specs/2026-09-14-windows-signed-audio-paths-design.md`

## Global Constraints

- Preserve Windows 10 support by accepting older APO initialization structures.
- The Windows 11 path must compile against a WDK/SDK that defines `IAudioSystemEffects3` and `APOInitSystemEffects3`.
- The current `AudioProcessingObject` INF class is already correct for the Windows 11 APO software component; do not regress it to `SoftwareComponent`.
- Do not claim Windows 11 production qualification merely because the DLL/INF builds or is attestation-signed.
- `loadedInstances` represents real processing instances only, never discovery-only instances.
- The existing app master-enable control and Windows effect-control state are independent gates; audio is processed only when both are enabled.
- Keep `APOProcess` real-time safe: no COM activation, property-store I/O, logging calls, allocation, locks, registry access, or process launches in `APOProcess`.
- CAPX property-store setup happens only during initialization/non-RT callbacks.
- The existing legacy runtime registry attachment may remain for development/Windows 10 compatibility, but it is not a Windows 11 production qualification path.
- Do not restore GitHub Actions.

---

### Task 1: Add testable APO policy helpers and a native unit-test executable

**Files:**
- Create: `native/windows/apo/VoxveilApoPolicy.h`
- Create: `native/windows/apo/tests/VoxveilApoPolicyTests.cpp`
- Create: `native/windows/apo/tests/VoxveilApoPolicyTests.vcxproj`
- Modify: `scripts/windows/build-windows.ps1`

**Interfaces:**
- Produces pure helpers for init-version classification, discovery counting, and effective DSP enablement.

- [ ] **Step 1: Write failing policy tests**

```cpp
#include "../VoxveilApoPolicy.h"
#include <cassert>

int wmain() {
    using namespace voxveil;
    assert(ClassifyInitSize(sizeof(APOInitSystemEffects)) == ApoInitFlavor::V1);
    assert(ClassifyInitSize(sizeof(APOInitSystemEffects2)) == ApoInitFlavor::V2);
    assert(ClassifyInitSize(sizeof(APOInitSystemEffects3)) == ApoInitFlavor::V3);
    assert(ClassifyInitSize(1) == ApoInitFlavor::Invalid);

    assert(ShouldCountLoadedInstance(ApoInitFlavor::V1, false));
    assert(ShouldCountLoadedInstance(ApoInitFlavor::V3, false));
    assert(!ShouldCountLoadedInstance(ApoInitFlavor::V3, true));

    assert(ShouldProcess(true, true, 50));
    assert(!ShouldProcess(false, true, 50));
    assert(!ShouldProcess(true, false, 50));
    assert(!ShouldProcess(true, true, 100));
    return 0;
}
```

- [ ] **Step 2: Run the test build and confirm failure**

```powershell
msbuild native/windows/apo/tests/VoxveilApoPolicyTests.vcxproj /m /p:Configuration=Release /p:Platform=x64
```

Expected: FAIL because `VoxveilApoPolicy.h` does not exist.

- [ ] **Step 3: Implement pure policy helpers**

```cpp
#pragma once
#include <audioenginebaseapo.h>
#include <audioengineextensionapo.h>

namespace voxveil {

enum class ApoInitFlavor { Invalid, V1, V2, V3 };

constexpr ApoInitFlavor ClassifyInitSize(UINT32 bytes) noexcept {
    if (bytes == sizeof(APOInitSystemEffects3)) return ApoInitFlavor::V3;
    if (bytes == sizeof(APOInitSystemEffects2)) return ApoInitFlavor::V2;
    if (bytes == sizeof(APOInitSystemEffects)) return ApoInitFlavor::V1;
    return ApoInitFlavor::Invalid;
}

constexpr bool ShouldCountLoadedInstance(ApoInitFlavor flavor, bool discoveryOnly) noexcept {
    return flavor != ApoInitFlavor::Invalid && !(flavor == ApoInitFlavor::V3 && discoveryOnly);
}

constexpr bool ShouldProcess(bool appEnabled, bool systemEffectEnabled, LONG vocalPercent) noexcept {
    return appEnabled && systemEffectEnabled && vocalPercent < 100;
}

} // namespace voxveil
```

- [ ] **Step 4: Make the test project a plain x64 console executable**

Use the same Windows SDK/WDK include environment as the APO project, `stdcpp17`, warnings as errors, and no APO DLL linkage.

- [ ] **Step 5: Run tests**

```powershell
msbuild native/windows/apo/tests/VoxveilApoPolicyTests.vcxproj /m /p:Configuration=Release /p:Platform=x64
native\windows\apo\tests\x64\Release\VoxveilApoPolicyTests.exe
```

Expected: exit 0.

- [ ] **Step 6: Add the native test to the Windows build verification phase**

`build-windows.ps1` builds and runs the test executable before building the production APO DLL.

- [ ] **Step 7: Commit**

```bash
git add native/windows/apo/VoxveilApoPolicy.h native/windows/apo/tests scripts/windows/build-windows.ps1
git commit -m "test(windows): add APO CAPX policy tests"
```

### Task 2: Bump shared-state ABI and add the OS effect-state gate

**Files:**
- Modify: `native/windows/apo/VoxveilSharedState.h`
- Modify: `native/windows/apo/VoxveilControl.cpp`
- Modify: `native/windows/apo/VoxveilControlCli.cpp`
- Modify: `native/windows/apo/VoxveilApo.cpp`
- Test: `native/windows/apo/tests/VoxveilApoPolicyTests.cpp`

**Interfaces:**
- Shared mapping becomes `Local\VoxveilApoControl-v2` / ABI 2.
- Adds `systemEffectEnabled` and `capxInstances` without changing the existing exported `VoxveilGetState` signature.

- [ ] **Step 1: Add policy tests for the independent gates**

Add assertions proving application and OS state cannot enable each other:

```cpp
assert(!ShouldProcess(true, false, 0));
assert(!ShouldProcess(false, true, 0));
assert(ShouldProcess(true, true, 0));
```

- [ ] **Step 2: Change the shared state atomically as one ABI revision**

```cpp
constexpr wchar_t kSharedStateName[] = L"Local\\VoxveilApoControl-v2";
constexpr LONG kSharedStateAbi = 2;

struct SharedState {
    volatile LONG abi;
    volatile LONG enabled;
    volatile LONG systemEffectEnabled;
    volatile LONG vocalPercent;
    volatile LONG heartbeat;
    volatile LONG loadedInstances;
    volatile LONG capxInstances;
};
```

Initialization defaults:

```cpp
InterlockedExchange(&state->enabled, 0);
InterlockedExchange(&state->systemEffectEnabled, 1);
InterlockedExchange(&state->vocalPercent, 100);
InterlockedExchange(&state->heartbeat, 0);
InterlockedExchange(&state->loadedInstances, 0);
InterlockedExchange(&state->capxInstances, 0);
```

- [ ] **Step 3: Keep the old control ABI stable**

`VoxveilGetState` continues returning app `enabled`, `vocalPercent`, `heartbeat`, and real `loadedInstances` exactly as before. Add a new optional export instead of changing the signature:

```cpp
extern "C" __declspec(dllexport) int __stdcall VoxveilGetCapxState(
    int* systemEffectEnabled,
    unsigned int* capxInstances) noexcept;
```

Add it to `VoxveilControl.def`.

- [ ] **Step 4: Extend CLI status without breaking the Rust parser**

Keep `loaded=<number>` unchanged and append:

```text
enabled=1 vocal=50 heartbeat=12 loaded=1 system-effect=1 capx=1
```

The Rust parser already searches `loaded=` and therefore remains compatible.

- [ ] **Step 5: Update APOProcess to use both gates**

Replace the current condition with reads performed before the frame loop:

```cpp
const bool appEnabled = InterlockedCompareExchange(&state_->enabled, 0, 0) != 0;
const bool systemEffectEnabled = InterlockedCompareExchange(&state_->systemEffectEnabled, 0, 0) != 0;
const LONG vocal = std::clamp<LONG>(
    InterlockedCompareExchange(&state_->vocalPercent, 0, 0), 0, 100);

if (voxveil::ShouldProcess(appEnabled, systemEffectEnabled, vocal) && channels >= 2) {
    // existing mid/side loop only
}
```

- [ ] **Step 6: Build and run tests**

```powershell
msbuild native/windows/apo/tests/VoxveilApoPolicyTests.vcxproj /m /p:Configuration=Release /p:Platform=x64
native\windows\apo\tests\x64\Release\VoxveilApoPolicyTests.exe
msbuild native/windows/apo/VoxveilControl.vcxproj /m /p:Configuration=Release /p:Platform=x64
msbuild native/windows/apo/VoxveilApo.vcxproj /m /p:Configuration=Release /p:Platform=x64
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add native/windows/apo/VoxveilSharedState.h native/windows/apo/VoxveilControl.cpp native/windows/apo/VoxveilControlCli.cpp native/windows/apo/VoxveilControl.def native/windows/apo/VoxveilApo.cpp native/windows/apo/tests/VoxveilApoPolicyTests.cpp
git commit -m "feat(windows): add APO system-effect state gate"
```

### Task 3: Implement `IAudioSystemEffects2` and `IAudioSystemEffects3`

**Files:**
- Modify: `native/windows/apo/VoxveilApo.h`
- Modify: `native/windows/apo/VoxveilApo.cpp`
- Create: `native/windows/apo/VoxveilApoIds.h`
- Test: `native/windows/apo/tests/VoxveilApoPolicyTests.cpp`

**Interfaces:**
- Produces one effect GUID, `GetEffectsList`, `GetControllableSystemEffectsList`, and `SetAudioSystemEffectState`.

- [ ] **Step 1: Generate and commit a stable Voxveil effect GUID**

Generate once with PowerShell `New-Guid`, then commit the literal in `VoxveilApoIds.h`:

```cpp
inline const GUID GUID_VoxveilVocalSuppressionEffect = { /* committed literal generated once */ };
```

Before committing, replace the comment with the generated numeric initializer. Never regenerate it after release.

- [ ] **Step 2: Extend the COM inheritance and map**

```cpp
#include <audioengineextensionapo.h>

class ATL_NO_VTABLE CVoxveilApo :
    public CComObjectRootEx<CComMultiThreadModel>,
    public CComCoClass<CVoxveilApo, &CLSID_VoxveilApo>,
    public CBaseAudioProcessingObject,
    public IAudioSystemEffects3
{
    // ...
    BEGIN_COM_MAP(CVoxveilApo)
        COM_INTERFACE_ENTRY(IAudioSystemEffects3)
        COM_INTERFACE_ENTRY(IAudioSystemEffects2)
        COM_INTERFACE_ENTRY(IAudioSystemEffects)
        COM_INTERFACE_ENTRY(IAudioProcessingObject)
        COM_INTERFACE_ENTRY(IAudioProcessingObjectRT)
        COM_INTERFACE_ENTRY(IAudioProcessingObjectConfiguration)
    END_COM_MAP()

    STDMETHOD(GetEffectsList)(LPGUID* effects, UINT* count, HANDLE event) override;
    STDMETHOD(GetControllableSystemEffectsList)(AUDIO_SYSTEMEFFECT** effects, UINT* count, HANDLE event) override;
    STDMETHOD(SetAudioSystemEffectState)(GUID effectId, AUDIO_SYSTEMEFFECT_STATE state) override;
};
```

- [ ] **Step 3: Implement legacy `GetEffectsList`**

Return the one effect GUID with `CoTaskMemAlloc` only when the effect is currently enabled and the processing mode is not `AUDIO_SIGNALPROCESSINGMODE_RAW`. Always initialize output pointers/counts before returning.

- [ ] **Step 4: Implement Windows 11 controllable-effect discovery**

```cpp
STDMETHODIMP CVoxveilApo::GetControllableSystemEffectsList(
    AUDIO_SYSTEMEFFECT** effects,
    UINT* count,
    HANDLE event) {
    if (effects == nullptr || count == nullptr) return E_POINTER;
    *effects = nullptr;
    *count = 0;
    ReplaceEffectsChangedEvent(event);
    if (IsEqualGUID(audioProcessingMode_, AUDIO_SIGNALPROCESSINGMODE_RAW)) return S_OK;

    auto* item = static_cast<AUDIO_SYSTEMEFFECT*>(CoTaskMemAlloc(sizeof(AUDIO_SYSTEMEFFECT)));
    if (item == nullptr) return E_OUTOFMEMORY;
    item->id = GUID_VoxveilVocalSuppressionEffect;
    item->canSetState = TRUE;
    item->state = CurrentSystemEffectState();
    *effects = item;
    *count = 1;
    return S_OK;
}
```

`ReplaceEffectsChangedEvent` duplicates the incoming handle with `DuplicateHandle`, closes the previous duplicate, and never stores the caller's raw handle directly.

- [ ] **Step 5: Implement OS effect-state changes**

```cpp
STDMETHODIMP CVoxveilApo::SetAudioSystemEffectState(
    GUID effectId,
    AUDIO_SYSTEMEFFECT_STATE state) {
    if (!IsEqualGUID(effectId, GUID_VoxveilVocalSuppressionEffect)) return E_NOTFOUND;
    const LONG enabled = state == AUDIO_SYSTEMEFFECT_STATE_ON ? 1 : 0;
    const LONG previous = InterlockedExchange(&state_->systemEffectEnabled, enabled);
    if (previous != enabled && effectsChangedEvent_ != nullptr) {
        SetEvent(effectsChangedEvent_);
    }
    return S_OK;
}
```

If `state_` is unavailable, return `E_UNEXPECTED` rather than dereferencing null.

- [ ] **Step 6: Close the duplicated event in the destructor**

Close only the duplicated handle owned by the APO.

- [ ] **Step 7: Build the APO and native policy test**

```powershell
msbuild native/windows/apo/tests/VoxveilApoPolicyTests.vcxproj /m /p:Configuration=Release /p:Platform=x64
native\windows\apo\tests\x64\Release\VoxveilApoPolicyTests.exe
msbuild native/windows/apo/VoxveilApo.vcxproj /m /p:Configuration=Release /p:Platform=x64
```

Expected: PASS with no missing pure virtual methods.

- [ ] **Step 8: Commit**

```bash
git add native/windows/apo/VoxveilApo.h native/windows/apo/VoxveilApo.cpp native/windows/apo/VoxveilApoIds.h native/windows/apo/tests/VoxveilApoPolicyTests.cpp
git commit -m "feat(windows): implement IAudioSystemEffects3"
```

### Task 4: Parse `APOInitSystemEffects3` correctly and stop discovery-only false readiness

**Files:**
- Modify: `native/windows/apo/VoxveilApo.h`
- Modify: `native/windows/apo/VoxveilApo.cpp`
- Test: `native/windows/apo/tests/VoxveilApoPolicyTests.cpp`

**Interfaces:**
- Initialization supports v1/v2/v3.
- `loadedInstances` increments only for a real processing instance.
- `capxInstances` counts successful v3 initialization separately.

- [ ] **Step 1: Move `loadedInstances` counting out of the constructor**

The constructor may open shared state but must not increment `loadedInstances`. Add members:

```cpp
bool countedLoadedInstance_ = false;
bool countedCapxInstance_ = false;
GUID audioProcessingMode_ = AUDIO_SIGNALPROCESSINGMODE_DEFAULT;
bool initializeForDiscoveryOnly_ = false;
```

- [ ] **Step 2: Validate initialization input by exact structure size**

```cpp
const auto flavor = voxveil::ClassifyInitSize(cbDataSize);
if (data == nullptr || flavor == voxveil::ApoInitFlavor::Invalid) {
    return E_INVALIDARG;
}
```

Retain `APOERR_ALREADY_INITIALIZED` handling.

- [ ] **Step 3: Parse v2/v3 processing mode**

For v2 and v3, read `AudioProcessingMode`; for v1 use `AUDIO_SIGNALPROCESSINGMODE_DEFAULT`.

For v3:

```cpp
const auto* init3 = reinterpret_cast<const APOInitSystemEffects3*>(data);
initializeForDiscoveryOnly_ = init3->InitializeForDiscoveryOnly != FALSE;
audioProcessingMode_ = init3->AudioProcessingMode;
```

- [ ] **Step 4: Count only real processing instances**

After all initialization validation succeeds:

```cpp
if (voxveil::ShouldCountLoadedInstance(flavor, initializeForDiscoveryOnly_)) {
    InterlockedIncrement(&state_->loadedInstances);
    countedLoadedInstance_ = true;
}
if (flavor == voxveil::ApoInitFlavor::V3) {
    InterlockedIncrement(&state_->capxInstances);
    countedCapxInstance_ = true;
}
```

The destructor decrements only counters whose matching boolean is true.

- [ ] **Step 5: Preserve discovery behavior without processing side effects**

A v3 discovery-only instance may answer `GetControllableSystemEffectsList`, but must not raise runtime readiness through `loadedInstances` and must not touch the real-time processing heartbeat.

- [ ] **Step 6: Build and run unit tests**

```powershell
msbuild native/windows/apo/tests/VoxveilApoPolicyTests.vcxproj /m /p:Configuration=Release /p:Platform=x64
native\windows\apo\tests\x64\Release\VoxveilApoPolicyTests.exe
msbuild native/windows/apo/VoxveilApo.vcxproj /m /p:Configuration=Release /p:Platform=x64
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add native/windows/apo/VoxveilApo.h native/windows/apo/VoxveilApo.cpp native/windows/apo/tests/VoxveilApoPolicyTests.cpp
git commit -m "fix(windows): exclude CAPX discovery instances from readiness"
```

### Task 5: Open the Windows 11 CAPX effects property store during v3 initialization

**Files:**
- Modify: `native/windows/apo/VoxveilApoIds.h`
- Modify: `native/windows/apo/VoxveilApo.h`
- Modify: `native/windows/apo/VoxveilApo.cpp`
- Modify: `native/windows/apo/VoxveilApo.vcxproj`

**Interfaces:**
- Adds stable `GUID_VoxveilApoPropertyContext`.
- Holds default/user/volatile `IPropertyStore` pointers only outside the RT callback.

- [ ] **Step 1: Generate one stable CAPX property-context GUID**

Generate once with `New-Guid` and commit the literal initializer in `VoxveilApoIds.h`:

```cpp
inline const GUID GUID_VoxveilApoPropertyContext = { /* committed literal generated once */ };
```

Replace the comment with the literal before commit.

- [ ] **Step 2: Add non-RT COM members**

```cpp
CComPtr<IPropertyStore> defaultStore_;
CComPtr<IPropertyStore> userStore_;
CComPtr<IPropertyStore> volatileStore_;
CComPtr<IAudioProcessingObjectLoggingService> loggingService_;
```

Include `audioengineextensionapo.h`, `mmdeviceapi.h`, `propvarutil.h`, and `servprov.h` as required by the installed SDK/WDK.

- [ ] **Step 3: Obtain optional Windows 11 logging service**

For v3 only:

```cpp
if (init3->pServiceProvider != nullptr) {
    (void)init3->pServiceProvider->QueryService(
        SID_AudioProcessingObjectLoggingService,
        IID_PPV_ARGS(&loggingService_));
}
```

Failure to obtain logging is non-fatal.

- [ ] **Step 4: Activate `IAudioSystemEffectsPropertyStore` from the endpoint**

Use the last device in `init3->pDeviceCollection` as Microsoft documents:

```cpp
UINT count = 0;
CComPtr<IMMDevice> endpoint;
RETURN_IF_FAILED(init3->pDeviceCollection->GetCount(&count));
if (count == 0) return E_UNEXPECTED;
RETURN_IF_FAILED(init3->pDeviceCollection->Item(count - 1, &endpoint));

PROPVARIANT activation{};
PropVariantInit(&activation);
RETURN_IF_FAILED(InitPropVariantFromCLSID(GUID_VoxveilApoPropertyContext, &activation));
CComPtr<IAudioSystemEffectsPropertyStore> effectsStore;
HRESULT hr = endpoint->Activate(
    __uuidof(IAudioSystemEffectsPropertyStore),
    CLSCTX_ALL,
    &activation,
    reinterpret_cast<void**>(&effectsStore));
PropVariantClear(&activation);
RETURN_IF_FAILED(hr);
```

Then open:

```cpp
RETURN_IF_FAILED(effectsStore->OpenDefaultPropertyStore(STGM_READ, &defaultStore_));
RETURN_IF_FAILED(effectsStore->OpenUserPropertyStore(STGM_READWRITE, &userStore_));
RETURN_IF_FAILED(effectsStore->OpenVolatilePropertyStore(STGM_READWRITE, &volatileStore_));
```

Use the exact method signatures in the installed WDK headers; if `CComPtr` address syntax differs, use temporary raw pointers and `Attach` without changing ownership semantics.

- [ ] **Step 5: Keep CAPX calls out of `APOProcess`**

No property-store getter/setter may be called by the RT method. The stores are established for compliant initialization/settings integration and future non-RT settings callbacks.

- [ ] **Step 6: Build against the target WDK**

```powershell
msbuild native/windows/apo/VoxveilApo.vcxproj /m /p:Configuration=Release /p:Platform=x64
```

Expected: PASS with `audioengineextensionapo.h` and `IAudioSystemEffectsPropertyStore` resolved.

- [ ] **Step 7: Commit**

```bash
git add native/windows/apo/VoxveilApoIds.h native/windows/apo/VoxveilApo.h native/windows/apo/VoxveilApo.cpp native/windows/apo/VoxveilApo.vcxproj
git commit -m "feat(windows): initialize APO CAPX property store"
```

### Task 6: Add CAPX property-store association to endpoint packaging without mixing legacy stores

**Files:**
- Modify: `native/windows/package/VoxveilApoExtension.inf.template`
- Modify: `scripts/windows/new-apo-extension-inf.ps1`
- Modify: `scripts/windows/install-system-audio-component.ps1`
- Modify: `native/windows/apo/VoxveilControlCli.cpp`
- Create: `scripts/quality/check-apo-capx-package.test.mjs`

**Interfaces:**
- Production Windows 11 extension packages use `FX\0\{VoxveilContext}\...` CAPX association.
- Legacy root `FX\0` runtime attachment remains development/Windows 10-only.

- [ ] **Step 1: Write an INF-generation invariant test first**

Generate an extension INF to a temp path and assert:

```js
assert.match(inf, /FX\\0\\%VOXVEIL_APO_CONTEXT%/i);
assert.match(inf, /PKEY_FX_Association/i);
assert.doesNotMatch(inf, /TODO|@@[A-Z_]+@@/);
```

Also assert a production/CAPX generation mode does not emit a root-level custom `HKR,FX\0,%PKEY_FX_Association%` entry.

- [ ] **Step 2: Add a stable context string to the template**

Add the literal GUID generated in Task 5:

```ini
VOXVEIL_APO_CONTEXT = "{<same committed context GUID>}"
```

The actual plan executor replaces `<same committed context GUID>` in this example with the literal already committed in `VoxveilApoIds.h`; the template and C++ must match byte-for-byte as GUID values before commit.

- [ ] **Step 3: Add explicit generator mode**

Extend `new-apo-extension-inf.ps1` with:

```powershell
[ValidateSet('Capx','Legacy')]
[string]$FxPropertyMode = 'Capx'
```

For `Capx`, emit the endpoint association in the signed extension binding section as:

```ini
HKR,FX\0\%VOXVEIL_APO_CONTEXT%,%PKEY_FX_Association%,,%KSNODETYPE_ANY%
HKR,FX\0,%PKEY_CompositeFX_StreamEffectClsid%,0x00010000,%VOXVEIL_SFX_CLSID%
HKR,FX\0,%PKEY_SFX_ProcessingModes_Supported_For_Streaming%,%REG_MULTI_SZ%,%AUDIO_SIGNALPROCESSINGMODE_DEFAULT%,%AUDIO_SIGNALPROCESSINGMODE_MEDIA%,%AUDIO_SIGNALPROCESSINGMODE_MOVIE%
```

For `Legacy`, preserve the existing root-level association used by the old fallback.

- [ ] **Step 4: Prevent production Windows 11 install from applying legacy runtime registry attachment**

When the installer is consuming prebuilt production-signed APO/extension catalogs, it must not call `voxveil-control.exe attach-effects` for a CAPX package. Require the signed extension INF to contain the CAPX context association. If only runtime-interface attachment is possible, fail with a clear message that this endpoint package is not CAPX production-bound.

Keep `attach-effects` only in explicitly development/test or legacy Windows 10 flows and label it as such in console output.

- [ ] **Step 5: Keep the CLI legacy command but rename/help-label it**

Do not delete the command needed for development. Its help/status text must say that direct `FX\0` registry attachment is legacy/development behavior and is not the Windows 11 CAPX production path.

- [ ] **Step 6: Run package tests**

```powershell
node --test scripts/quality/check-apo-capx-package.test.mjs
npm run quality
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add native/windows/package/VoxveilApoExtension.inf.template scripts/windows/new-apo-extension-inf.ps1 scripts/windows/install-system-audio-component.ps1 native/windows/apo/VoxveilControlCli.cpp scripts/quality/check-apo-capx-package.test.mjs
git commit -m "feat(windows): add CAPX APO property association"
```

### Task 7: Verify runtime readiness and effect discovery on Windows 10 and Windows 11

**Files:**
- Create: `scripts/windows/probe-apo-capx.ps1`
- Modify: `crates/voxveil-windows-audio/src/relay.rs`
- Test: `crates/voxveil-windows-audio/src/relay.rs`

**Interfaces:**
- Diagnostic probe reports `loaded`, `capx`, and system-effect state without changing the backend-ready rule.

- [ ] **Step 1: Keep readiness based on real `loaded=` instances**

Add a Rust parser test proving extra status fields do not alter the existing loaded-instance parse:

```rust
#[test]
fn parses_loaded_count_from_capx_status() {
    let status = "enabled=1 vocal=30 heartbeat=99 loaded=1 system-effect=1 capx=2";
    assert_eq!(parse_loaded_instances(status), Some(1));
}
```

Also test discovery-only status:

```rust
let status = "enabled=0 vocal=100 heartbeat=0 loaded=0 system-effect=1 capx=1";
assert_eq!(parse_loaded_instances(status), Some(0));
```

- [ ] **Step 2: Add a non-mutating diagnostic script**

`probe-apo-capx.ps1` runs `voxveil-control.exe status`, parses:

```text
loaded
capx
system-effect
heartbeat
```

and emits JSON. It must not install packages, write registry values, restart AudioSrv, or enable/disable effects.

- [ ] **Step 3: Run Rust tests**

```powershell
cargo test -p voxveil-windows-audio relay::tests
```

Expected: PASS.

- [ ] **Step 4: Validate Windows 10 compatibility**

On Windows 10, confirm the APO initializes through v1/v2, `loaded>=1` on a processing graph, `capx=0`, and processing still obeys app master/vocal controls.

- [ ] **Step 5: Validate Windows 11 discovery behavior**

On Windows 11, query audio effects through a small diagnostic or Windows API client so discovery occurs. Confirm `capx` may increase during v3 initialization while `loaded` remains 0 for discovery-only instances. Then start real playback and confirm `loaded>=1`.

- [ ] **Step 6: Commit**

```bash
git add scripts/windows/probe-apo-capx.ps1 crates/voxveil-windows-audio/src/relay.rs
git commit -m "test(windows): distinguish CAPX discovery from APO readiness"
```

### Task 8: Add Windows 11 HLK/support validation gate and release documentation

**Files:**
- Create: `docs/testing/windows-apo-capx-hlk.md`
- Create: `docs/release/windows-apo-production-gate.md`
- Modify: `docs/specs/platform/windows.md`
- Modify: `README.md`

**Interfaces:**
- Documentation prevents an implementation/build success from being mislabeled production-qualified.

- [ ] **Step 1: Write the HLK test matrix**

Record at minimum:

```text
Windows 11 build / WDK / HLK versions
APO package/catalog hashes
Underlying endpoint hardware ID and driver version
IAudioSystemEffects3 query succeeds
APOInitSystemEffects3 received for Windows 11 path
Discovery-only instance does not increment loadedInstances
GetControllableSystemEffectsList returns Voxveil effect
SetAudioSystemEffectState OFF bypasses DSP without disabling the app master state
SetAudioSystemEffectState ON restores DSP when app master is enabled
RAW processing mode exposes no controllable Voxveil effect
Applicable Windows 11 audio/APO HLK playlist result
```

- [ ] **Step 2: Document the external Microsoft support question**

The release gate must require a resolved answer for Voxveil's ISV/componentized-APO distribution model, specifically whether the intended arbitrary third-party endpoint extension package can be distributed and signed in the planned Hardware Dev Center path.

- [ ] **Step 3: Define production gate conditions**

Tier 3 may be labeled production-ready only when all are true:

```markdown
- [ ] Production-signed APO and endpoint extension package exists.
- [ ] Package uses CAPX context association, not runtime registry attachment.
- [ ] Applicable Windows 11 HLK APO tests pass or Microsoft provides an explicit accepted exception.
- [ ] Microsoft support/Partner Center distribution path is confirmed.
- [ ] HDAudio, USB, Bluetooth, HDMI/DisplayPort, and docked outputs have representative real-device validation.
- [ ] Windows 10 compatibility regression matrix passes where Windows 10 remains supported.
```

- [ ] **Step 4: Update architecture docs**

README and `windows.md` must describe the APO as available/advanced until this gate is satisfied. Do not replace Tier 1/Tier 2 fallback paths merely because `IAudioSystemEffects3` has been implemented.

- [ ] **Step 5: Run full Windows checks**

```powershell
cargo test --workspace
npm test
npm run typecheck
npm run quality
npm run build:windows
```

Expected: PASS on a configured Windows development machine.

- [ ] **Step 6: Commit**

```bash
git add docs/testing/windows-apo-capx-hlk.md docs/release/windows-apo-production-gate.md docs/specs/platform/windows.md README.md
git commit -m "docs(windows): gate APO production on CAPX HLK validation"
```

Tier 3 implementation is code-complete after the automated and Windows 10/11 functional checks pass, but Tier 3 is **not production-qualified** until the external Microsoft/HLK release gate is satisfied.