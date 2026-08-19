# Native Windows Topology Resolver Design

## Goal

Replace Voxveil's INF-first Windows playback binding resolver with a runtime-first resolver that can correctly handle OEM audio stacks such as Realtek, USB audio, HDMI, and other devices whose installed INF does not expose a directly parseable `KSCATEGORY_TOPOLOGY` `AddInterface` reference.

The user-facing result is that Voxveil should resolve the active playback endpoint automatically and only report `Unavailable` when Windows itself does not expose a safe binding or the required installation metadata cannot be recovered safely.

## Problem

The current resolver starts with a Core Audio endpoint, walks to a parent PnP device, reads one installed driver INF, and searches the INF text for a topology `AddInterface` reference. On some OEM stacks, including the reported Realtek XU endpoint, the endpoint and adapter are discovered but the installed INF does not yield a unique topology reference. Voxveil therefore reports:

`No unambiguous topology AddInterface reference was found in the installed audio driver INF.`

That is a resolver failure, not proof that the endpoint lacks a topology interface.

## Decision

Use Windows runtime topology/device-interface APIs as the primary source of truth. Retain INF/driver-package parsing only as a compatibility fallback and as the final source for a literal reference string when Windows identifies the correct interface but does not expose that string directly through the user-mode API surface.

## Runtime Resolution Pipeline

### 1. Enumerate playback endpoints

Keep the existing Core Audio endpoint enumeration. Each endpoint must retain:

- endpoint ID
- friendly name
- default-device flag

The browser/UI receives only opaque endpoint identity and safe status metadata. Hardware IDs, interface paths, and topology reference strings remain native-only.

### 2. Traverse from endpoint to hardware topology

For each playback `IMMDevice`:

1. call `IMMDevice::Activate(IID_IDeviceTopology)`
2. get endpoint connector `0`
3. call `IConnector::GetConnectedTo` to cross into the hardware adapter topology
4. query the connected connector for `IPart`
5. call `IPart::GetTopologyObject` to obtain the adapter `IDeviceTopology`
6. call `IDeviceTopology::GetDeviceId` (or `IConnector::GetDeviceIdConnectedTo` when sufficient) to obtain the connected audio-device identifier

This follows Microsoft's documented `GetHardwareDeviceTopology` pattern instead of relying on endpoint display names or the first PnP ancestor with a driver.

Failure to activate or traverse topology is not fatal to discovery; it moves the endpoint to the runtime device-interface/PnP correlation path and then the INF fallback.

### 3. Enumerate active topology interfaces

Enumerate present device interfaces in `KSCATEGORY_TOPOLOGY` (`{DDA54A40-1E4C-11D1-A050-405705C10000}`) using Windows SetupAPI/Configuration Manager APIs.

For each candidate, collect native-only metadata sufficient to correlate it with the endpoint adapter:

- device interface symbolic link (opaque; never parsed)
- interface class GUID
- owning devnode / instance ID
- device container or parent relationship where available
- alias relationship to other interfaces when exposed by Windows

The device path returned by SetupAPI must be treated as opaque. Voxveil must not infer a reference string by splitting or pattern-matching that path.

### 4. Correlate endpoint and topology interface

Rank candidates using only deterministic Windows relationships:

1. same hardware topology device reached through the DeviceTopology traversal
2. same owning devnode/device identity
3. device-interface alias relationship when Windows reports that two interfaces are aliases
4. same unambiguous device/container ancestry
5. exact PnP identity correlation

A single surviving candidate becomes the runtime topology interface.

If zero runtime candidates survive, use the INF fallback.

If more than one runtime candidate remains and Windows does not expose a deterministic distinction, classify the endpoint as `Ambiguous`; do not guess.

### 5. Reference-string handling

Use `SetupDiGetDeviceInterfaceAlias` or the equivalent Configuration Manager alias API to prove that two interfaces belong to the same device and use the same reference string. Alias APIs are correlation evidence; they are not used to parse the literal string from the symbolic link.

For installation, the componentized extension INF still requires the literal reference string. Recover that literal only from trusted driver/package metadata tied to the exact runtime-resolved devnode/interface. The fallback parser may inspect the primary INF and related component/extension INFs for that same device.

Rules:

- never parse a reference string from a device path
- never accept an INF candidate for a different device just because its text looks similar
- if exactly one reference string is recovered for the runtime-resolved interface, use it
- if none can be recovered, keep the topology interface resolved but report that installation metadata is unavailable
- if several remain, classify as `Ambiguous`

This distinction matters for OEM drivers: Voxveil can know which topology interface is correct even when it cannot yet generate a safe extension INF for that interface.

### 6. INF/driver-package fallback

Keep `scripts/windows/discover-system-audio-endpoints.ps1` only as fallback/diagnostic logic during this transition.

Fallback rules:

- inspect the installed primary INF plus related component/extension package metadata for the same resolved device
- accept a topology reference only when exactly one candidate matches the already-resolved device/interface
- never change the target device based solely on a string match
- never convert ambiguity into an arbitrary choice

The result must report its resolution source (`runtime`, `runtime+metadata`, `fallback`, or `none`).

## Native Module Boundaries

Add dedicated native resolver modules under `crates/voxveil-windows-audio/src/`, rather than growing `discovery.rs` indefinitely.

Responsibilities:

- `topology.rs`: Core Audio `IDeviceTopology` traversal and hardware audio-device identity
- `device_interfaces.rs`: SetupAPI/CfgMgr32 enumeration, devnode ownership, and alias correlation
- `discovery.rs`: orchestration, classification, package availability, metadata fallback, and diagnostics

Public browser-facing Tauri DTOs remain unchanged except for optional safe diagnostics such as resolution source (`runtime`, `runtime+metadata`, `fallback`, `none`).

## Status Classification

`Ready`
: Voxveil APO is already loaded for the resolved output.

`Installable`
: unique safe topology binding and literal reference string are resolved, and a matching signed Voxveil extension package is present.

`ComponentRequired`
: the runtime topology binding is resolved but either the matching signed extension package is missing or package-generation metadata is not yet available.

`Ambiguous`
: Windows exposes multiple plausible topology bindings/reference strings and no deterministic relationship selects one.

`Unsupported`
: no safe runtime/fallback topology binding can be established.

For the reported Realtek XU endpoint, the current `Unsupported`/`Unavailable` state should become at least `ComponentRequired` if runtime topology correlation succeeds, and `Installable` when the literal binding metadata and matching package are both available.

## Installation Safety

The existing opaque endpoint-ID installation boundary remains.

Before elevation/install, native code must re-run endpoint resolution and freeze a descriptor containing the exact resolved device identity, topology interface identity, reference string, and matching package identity. The elevated installer must validate that the descriptor still refers to the same present device before installing anything.

If the endpoint disappeared, changed driver, changed topology binding, changed package identity, or became ambiguous, installation aborts.

## Packaging and Signing

Runtime discovery does not remove Microsoft's requirement for a matching signed device-extension package. The resolver determines the correct endpoint/device/topology binding automatically; package matching/signing remains a separate gate.

Voxveil must verify that the packaged extension INF/CAT corresponds to the selected hardware ID and resolved topology reference before exposing `Install`.

## Error Handling

- DeviceTopology activation/traversal failure -> attempt runtime device-interface/PnP correlation, then metadata fallback.
- SetupAPI/CfgMgr32 enumeration failure -> metadata fallback with diagnostic detail.
- Multiple runtime candidates -> `Ambiguous`, no fallback guess.
- Unique runtime topology interface but no recoverable literal reference string -> `ComponentRequired` with a specific diagnostic, not generic `Unsupported`.
- Driver changes between discovery and install -> abort and refresh.
- Missing matching signed package -> `ComponentRequired`, not `Unsupported`.

## Tests

### Unit tests

Add pure tests for candidate correlation/classification:

- unique same-hardware-device topology candidate wins
- alias-matched topology candidate wins over unrelated candidates
- two indistinguishable topology candidates remain ambiguous
- zero runtime candidates falls back to metadata
- runtime-resolved device cannot be replaced by an INF-only candidate from another device
- unique runtime topology with no literal reference string becomes `ComponentRequired`
- unique runtime binding with reference string but no package becomes `ComponentRequired`
- unique runtime binding with matching package becomes `Installable`

### Windows integration tests

On a Windows runner:

- compile DeviceTopology/SetupAPI/CfgMgr32 calls
- enumerate topology interfaces without crashing when no physical audio endpoint exists
- empty endpoint input remains valid
- resolver returns structured diagnostics rather than throwing on unavailable hardware
- device paths are carried as opaque strings and never parsed for reference strings

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
2. Implement DeviceTopology endpoint-to-adapter traversal.
3. Implement SetupAPI/CfgMgr32 topology-interface enumeration.
4. Implement deterministic device/alias correlation.
5. Integrate literal reference-string recovery only for the resolved interface.
6. Keep existing INF logic as constrained fallback.
7. Update diagnostics/UI status copy.
8. Run full Windows build and package smoke test.
9. Ship a new ZIP for real-device validation on the reported Realtek XU endpoint.

## Non-goals

- guessing between multiple topology bindings
- parsing Windows device-interface paths
- disabling Windows driver-signing requirements
- modifying arbitrary OEM driver packages
- silently attaching Voxveil to every audio endpoint
- exposing hardware IDs/reference strings back to the browser UI

## Reference APIs

- Microsoft `IDeviceTopology` / DeviceTopology API
- Microsoft `IConnector::GetConnectedTo` / `GetDeviceIdConnectedTo`
- Microsoft `KSCATEGORY_TOPOLOGY`
- Microsoft `SetupDiGetDeviceInterfaceAlias`
- Microsoft Configuration Manager device-interface APIs (`CM_Get_Device_Interface_*`)
