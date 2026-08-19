# Native Windows Topology Resolver Design

## Goal

Replace Voxveil's INF-first Windows playback binding resolver with a runtime-first resolver that can correctly handle OEM audio stacks such as Realtek, USB audio, HDMI, and other devices whose installed INF does not expose a directly parseable `KSCATEGORY_TOPOLOGY` `AddInterface` reference.

The user-facing result is that Voxveil should resolve the active playback endpoint automatically and only report `Unavailable` when Windows itself does not expose a safe unique topology binding.

## Problem

The current resolver starts with a Core Audio endpoint, walks to a parent PnP device, reads one installed driver INF, and searches the INF text for a topology `AddInterface` reference. On some OEM stacks, including the reported Realtek XU endpoint, the endpoint and adapter are discovered but the installed INF does not yield a unique topology reference. Voxveil therefore reports:

`No unambiguous topology AddInterface reference was found in the installed audio driver INF.`

That is a resolver failure, not proof that the endpoint lacks a topology interface.

## Decision

Use Windows runtime topology/device-interface APIs as the primary source of truth. Retain INF parsing only as a compatibility fallback and diagnostic source.

## Runtime Resolution Pipeline

### 1. Enumerate playback endpoints

Keep the existing Core Audio endpoint enumeration. Each endpoint must retain:

- endpoint ID
- friendly name
- default-device flag

The browser/UI receives only opaque endpoint identity and safe status metadata. Hardware IDs, interface paths, and topology reference strings remain native-only.

### 2. Activate `IDeviceTopology`

For each playback `IMMDevice`, call `IMMDevice::Activate(IID_IDeviceTopology)`.

Use the endpoint topology connector to traverse from the endpoint to the connected adapter topology. The resolver must identify the adapter/devnode associated with the playback path instead of relying only on endpoint display names or the first PnP ancestor with a driver.

Failure to activate or traverse topology is not fatal to discovery; it moves the endpoint to the fallback resolver.

### 3. Enumerate active topology interfaces

Enumerate present device interfaces in `KSCATEGORY_TOPOLOGY` (`{DDA54A40-1E4C-11D1-A050-405705C10000}`) using Windows SetupAPI/Configuration Manager APIs.

For each candidate, collect native-only metadata sufficient to correlate it with the endpoint adapter:

- device interface symbolic link
- interface class GUID
- owning devnode / instance ID
- device container or parent relationship where available
- reference-string identity where derivable through alias APIs

Do not infer compatibility from the textual shape of the symbolic link.

### 4. Correlate endpoint and topology interface

Rank candidates using only deterministic Windows relationships:

1. same adapter/devnode reached through DeviceTopology traversal
2. same device/container ancestry where Windows exposes an unambiguous relationship
3. device-interface alias relationship when the endpoint/render/audio interface and topology interface share the same underlying device and reference string
4. exact PnP identity correlation

A single surviving candidate becomes the runtime topology binding.

If zero runtime candidates survive, use the INF fallback.

If more than one runtime candidate remains and Windows does not expose a deterministic distinction, classify the endpoint as `Ambiguous`; do not guess.

### 5. Reference string handling

Prefer device-interface alias APIs (`SetupDiGetDeviceInterfaceAlias` or the equivalent Configuration Manager alias API) when a corresponding audio/render interface is available. Windows defines aliased interfaces as interfaces exposed by the same device with the same reference string, so this is stronger evidence than parsing the OEM INF.

If the runtime API produces a topology interface path but the reference string cannot be safely reconstructed, the resolver may use the INF fallback only to recover the reference string for the already-identified devnode/interface. It must not use a different INF candidate to override the runtime device correlation.

### 6. INF fallback

Keep `scripts/windows/discover-system-audio-endpoints.ps1` only as a fallback/diagnostic path during this transition.

Fallback rules:

- scan the installed primary INF and related driver-package data available for the same resolved device
- accept a topology reference only when exactly one candidate matches the already-resolved device
- never change the target device based solely on a string match
- never convert ambiguity into an arbitrary choice

The fallback must report whether its result came from runtime APIs or INF parsing.

## Native Module Boundaries

Add a dedicated native resolver module under `crates/voxveil-windows-audio/src/`, rather than growing `discovery.rs` indefinitely.

Suggested responsibilities:

- `topology.rs`: Core Audio `IDeviceTopology` traversal and adapter identity
- `device_interfaces.rs`: SetupAPI/CfgMgr32 enumeration and alias correlation
- `discovery.rs`: orchestration, classification, package availability, and fallback selection

Public browser-facing Tauri DTOs remain unchanged except for optional safe diagnostics such as resolution source (`runtime`, `fallback`, `none`).

## Status Classification

`Ready`
: Voxveil APO is already loaded for the resolved output.

`Installable`
: unique safe topology binding resolved and a matching signed Voxveil extension package is present.

`ComponentRequired`
: unique safe topology binding resolved but the packaged signed extension does not match that endpoint.

`Ambiguous`
: Windows exposes multiple plausible topology bindings and no deterministic relationship selects one.

`Unsupported`
: no safe binding can be established by runtime APIs or fallback.

The specific Realtek failure in the current UI should become `ComponentRequired` or `Installable` if runtime topology correlation succeeds.

## Installation Safety

The existing opaque endpoint-ID installation boundary remains.

Before elevation/install, native code must re-run endpoint resolution and freeze a descriptor containing the exact resolved device identity and topology binding. The elevated installer must validate that the descriptor still refers to the same present device before installing anything.

If the endpoint disappeared, changed driver, changed topology binding, or became ambiguous, installation aborts.

## Packaging and Signing

Runtime discovery does not remove Microsoft's requirement for a matching signed device-extension package. The resolver only determines the correct endpoint/device/topology binding automatically.

Voxveil must still verify that the packaged extension INF/CAT corresponds to the selected hardware ID and resolved topology reference before exposing `Install`.

## Error Handling

- DeviceTopology activation failure -> attempt runtime interface/PnP correlation, then INF fallback.
- SetupAPI/CfgMgr32 enumeration failure -> INF fallback with diagnostic detail.
- Multiple runtime candidates -> `Ambiguous`, no fallback guess.
- Unique runtime device but missing safe reference string -> use INF only to recover that same device's reference; otherwise `Unsupported`.
- Driver changes between discovery and install -> abort and refresh.
- Missing matching signed package -> `ComponentRequired`, not `Unsupported`.

## Tests

### Unit tests

Add pure tests for candidate correlation/classification:

- unique same-devnode topology candidate wins
- alias-matched topology candidate wins over unrelated candidates
- two indistinguishable topology candidates remain ambiguous
- zero runtime candidates falls back to INF
- runtime-resolved device cannot be replaced by an INF-only candidate from another device
- unique runtime binding with no package becomes `ComponentRequired`
- unique runtime binding with matching package becomes `Installable`

### Windows integration tests

On a Windows runner:

- compile all DeviceTopology/SetupAPI/CfgMgr32 calls
- enumerate topology interfaces without crashing when no physical audio endpoint exists
- empty endpoint input remains valid
- resolver returns structured diagnostics rather than throwing on unavailable hardware

### Existing regression suites

Run:

- `cargo test -p voxveil-windows-audio`
- `cargo test -p voxveil-tauri`
- UI tests
- TypeScript typecheck
- quality tests
- full `scripts/windows/build-windows.ps1`
- packaged executable smoke test

## Rollout

1. Add runtime candidate model + RED unit tests.
2. Implement SetupAPI/CfgMgr32 topology enumeration.
3. Add DeviceTopology adapter correlation.
4. Integrate alias/reference-string resolution.
5. Keep INF resolver as fallback.
6. Update diagnostics/UI status copy if needed.
7. Run full Windows build and package smoke test.
8. Ship a new ZIP for real-device validation on the reported Realtek XU endpoint.

## Non-goals

- guessing between multiple topology bindings
- disabling Windows driver-signing requirements
- modifying arbitrary OEM driver packages
- silently attaching Voxveil to every audio endpoint
- exposing hardware IDs/reference strings back to the browser UI

## Reference APIs

- Microsoft `IDeviceTopology` / DeviceTopology API
- Microsoft `KSCATEGORY_TOPOLOGY`
- Microsoft `SetupDiGetDeviceInterfaceAlias`
- Microsoft Configuration Manager device-interface APIs (`CM_Get_Device_Interface_*`)
