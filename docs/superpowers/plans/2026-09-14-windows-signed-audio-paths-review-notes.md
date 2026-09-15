# Windows Signed Audio Paths — Plan Self-Review Corrections

Date: 2026-09-14
Status: Authoritative corrections for execution

These corrections were found during the required post-plan self-review. When this file conflicts with any of the three tier plans, **this file is authoritative**. An executor must read this file before starting Tier 1, Tier 2, or Tier 3.

Applies to:

- `docs/superpowers/plans/2026-09-14-windows-tier1-vb-cable-relay.md`
- `docs/superpowers/plans/2026-09-14-windows-tier2-signed-virtual-driver.md`
- `docs/superpowers/plans/2026-09-14-windows-tier3-apo-capx.md`

## Execution order

Execute the plans sequentially in this order:

1. Tier 1 VB-CABLE relay.
2. Tier 2 signed virtual-driver tooling/runtime integration.
3. Tier 3 APO CAPX modernization.

Tier 2 consumes the Tier 1 relay abstraction. Tier 3 and Tier 2 both touch Windows release/docs files, so sequential execution avoids unnecessary merge conflicts.

## Tier 1 corrections

### 1. Do not use `DeviceEnumerator::get_device` with the pinned `wasapi = 0.23.0`

Task 5, Step 3 of the Tier 1 plan is replaced by the following.

Resolve active render devices from the same render collection already used by `relay.rs`:

```rust
fn active_render_device_by_id(
    enumerator: &DeviceEnumerator,
    wanted_id: &str,
) -> Result<wasapi::Device, String> {
    let collection = enumerator
        .get_device_collection(&Direction::Render)
        .map_err(|error| error.to_string())?;

    for device in &collection {
        let device = device.map_err(|error| error.to_string())?;
        let id = device.get_id().map_err(|error| error.to_string())?;
        if id == wanted_id {
            return Ok(device);
        }
    }

    Err(format!("active render endpoint not found: {wanted_id}"))
}
```

Then:

```rust
let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
let source_device = active_render_device_by_id(&enumerator, &spec.source_endpoint_id)?;
let physical_device = active_render_device_by_id(&enumerator, &spec.physical_output_endpoint_id)?;
```

Reason: the project currently pins `wasapi 0.23.0`; use the already-proven collection enumeration path rather than depending on direct ID lookup behavior from that version. Do not upgrade `wasapi` as part of Tier 1 solely to obtain direct lookup.

### 2. Make the Tier 1 React prop contract explicit

Task 9 must extend `SystemAudioEndpointsProps` with all route state/actions instead of reading global state inside the component:

```ts
interface SystemAudioEndpointsProps {
  endpoints: SystemAudioEndpoint[];
  backendStatus: ProcessingBackendStatus;
  backendKind: WindowsInterceptionKind | null;
  physicalOutputs: AudioOutput[];
  selectedPhysicalOutputId: string | null;
  busy: boolean;
  error: string | null;
  onInstall: (endpointId: string) => void;
  onSelectPhysicalOutput: (endpointId: string) => void;
  onGetVbCable: () => void;
  onOpenSoundSettings: () => void;
}
```

`HomeScreen.tsx` passes these values/actions from `useVoxveilState`; the panel stays a presentational component.

## Tier 2 corrections

### 1. Stable driver GUIDs are fixed now

Do not generate new GUIDs during implementation. Use these committed identities everywhere the Tier 2 plan asks for generated Voxveil driver GUIDs:

```cpp
// {79E4E58C-9714-44E8-ACA1-426F24B7A1E9}
DEFINE_GUID(
    GUID_DEVINTERFACE_VOXVEIL_VIRTUAL_AUDIO,
    0x79e4e58c, 0x9714, 0x44e8, 0xac, 0xa1, 0x42, 0x6f, 0x24, 0xb7, 0xa1, 0xe9);

// {3F67F54B-DCF8-4CB9-9E0E-79A13CFD046B}
DEFINE_GUID(
    KSNODETYPE_VOXVEIL_VIRTUAL_SPEAKER,
    0x3f67f54b, 0xdcf8, 0x4cb9, 0x9e, 0x0e, 0x79, 0xa1, 0x3c, 0xfd, 0x04, 0x6b);
```

### 2. Attestation CAB layout is a subfolder, not CAB-root files

Tier 2 Task 5, Step 2 is replaced.

The DDF must use a driver-package subfolder:

```text
.Set DestinationDir=VoxveilVirtualAudio
<path>\VoxveilVirtualAudio.inf
<path>\VoxveilVirtualAudio.sys
<path>\VoxveilVirtualAudio.cat
<path>\VoxveilVirtualAudio.pdb
```

The CAB must therefore contain:

```text
VoxveilVirtualAudio/
  VoxveilVirtualAudio.inf
  VoxveilVirtualAudio.sys
  VoxveilVirtualAudio.cat
  VoxveilVirtualAudio.pdb
```

The current Microsoft attestation documentation says files must not be at the CAB root and that the PDB is required for automated crash analysis. Update Task 4 package validation to allow/require the matching PDB in the submission package. The **runtime/release** package still excludes PDB unless the release packaging policy separately includes symbols.

### 3. The CAB must be EV-signed outside the repository before Partner Center submission

Repository scripts prepare the CAB but do not access the EV private key.

Tier 2 release documentation must add this external step between CAB generation and Partner Center upload:

```text
Use the EV certificate provider's approved signing process to Authenticode-sign the generated CAB with SHA-256 and a trusted timestamp, then upload that signed CAB to Hardware Dev Center.
```

Do not add certificate thumbprints, provider account identifiers, tokens, or signing secrets to repository scripts.

### 4. Do not label attestation as retail production certification

Microsoft's current driver-signing documentation describes attestation signing as a testing-scenario path; it does not make a package Windows Certified and it cannot be published to Windows Update for retail audiences.

Therefore Tier 2 must distinguish two release states:

```text
attestation-signed -> trusted pilot/direct-validation package; not Windows Certified
WHCP/HLK-signed or explicitly Microsoft-approved distribution path -> eligible for Voxveil retail-production gate
```

`stage-signed-virtual-driver.ps1` may stage a verified attestation-signed package only when an explicit non-retail/pilot flag is supplied, for example:

```powershell
-ReleaseChannel Pilot
```

For `-ReleaseChannel Retail`, the staging script must require the release metadata/evidence chosen for the WHCP/HLK or Microsoft-confirmed production path and refuse a package known only to be attestation-signed.

This does **not** remove the requested attestation tooling; it prevents the repository from misrepresenting its current Microsoft status.

## Tier 3 corrections

### 1. Stable APO GUIDs are fixed now

Use these exact identities instead of generating them during implementation:

```cpp
// {B9FD554E-8F72-4B20-9AB1-13F8E8BFDD02}
inline const GUID GUID_VoxveilVocalSuppressionEffect =
{ 0xb9fd554e, 0x8f72, 0x4b20, { 0x9a, 0xb1, 0x13, 0xf8, 0xe8, 0xbf, 0xdd, 0x02 } };

// {63E268CE-4CBC-48E0-BEB6-55103316F477}
inline const GUID GUID_VoxveilApoPropertyContext =
{ 0x63e268ce, 0x4cbc, 0x48e0, { 0xbe, 0xb6, 0x55, 0x10, 0x33, 0x16, 0xf4, 0x77 } };
```

The Extension INF string value for the context is exactly:

```ini
VOXVEIL_APO_CONTEXT = "{63E268CE-4CBC-48E0-BEB6-55103316F477}"
```

### 2. Do not introduce WIL-only error macros into the existing ATL APO

Where the Tier 3 plan examples use `RETURN_IF_FAILED`, implement explicit HRESULT handling because the current Voxveil APO does not depend on WIL:

```cpp
HRESULT hr = init3->pDeviceCollection->GetCount(&count);
if (FAILED(hr)) {
    return hr;
}
```

Repeat this pattern for `Item`, `InitPropVariantFromCLSID`, `Activate`, and property-store open calls. Keep `PropVariantClear` on every path after successful variant initialization.

### 3. Windows 10 compatibility is code-path compatibility, not a new package in this plan

The current `VoxveilApo.inf` targets the Windows 11 `AudioProcessingObject` class and its current target decoration starts at build 22621. Tier 3 must **not** silently invent a Windows 10 `SoftwareComponent` package.

Replace Tier 3 Task 7's live Windows 10 install requirement with:

- unit/native tests for v1/v2 initialization-size handling;
- compile-time compatibility with `APOInitSystemEffects` and `APOInitSystemEffects2`;
- a live Windows 10 APO install test only if a separately designed/signed Windows 10 package is later added.

Tier 1 remains the Windows 10 production fallback in this scope.

### 4. Discovery-only readiness rule is mandatory

`capxInstances` may count v3 discovery instances for diagnostics, but `loadedInstances` must be incremented only after successful non-discovery initialization. Rust readiness continues to use only `loaded=`.

## Final verification before implementation handoff

Before claiming any tier complete, execute the relevant plan checks plus:

```powershell
cargo test --workspace
npm test
npm run typecheck
npm run quality
```

On Windows, run `npm run build:windows` after native changes.

Real audio/driver/APO behavior still requires the hardware/OS matrices documented in each plan; GitHub source inspection alone cannot validate those runtime properties.