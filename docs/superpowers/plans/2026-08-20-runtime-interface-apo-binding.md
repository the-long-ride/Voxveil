# Runtime Interface APO Binding Implementation Plan

**Goal:** Make Windows system-audio endpoint binding work when OEM INFs (notably some Realtek packages) do not expose a usable `AddInterface` reference string.

**Architecture:** Treat the exact SetupAPI device-interface paths resolved at runtime as the primary binding. Keep those paths opaque, retain the `KSCATEGORY_AUDIO` alias path for the selected `KSCATEGORY_TOPOLOGY` interface, and carry both through endpoint discovery into the elevated installer. Install the componentized APO package as before, but attach FX properties at runtime by opening the exact interface registry keys with SetupAPI. Keep OEM-INF `AddInterface` parsing only as a fallback for devices where runtime topology resolution is unavailable.

**Tech Stack:** Rust (`windows` crate), Win32 SetupAPI/registry APIs in the existing C++ control CLI, PowerShell packaging/install scripts, Node quality tests.

---

## Task 1: Lock the new runtime-binding contract with tests

**Files:**
- Modify: `crates/voxveil-windows-audio/src/binding.rs`
- Modify: `crates/voxveil-windows-audio/src/device_interfaces.rs`
- Create: `scripts/quality/system-audio-runtime-interface-binding.test.mjs`

1. Add a failing Rust test proving a unique runtime binding does not require an INF topology reference.
2. Add a failing Rust test proving the selected topology candidate retains its exact topology interface path and audio alias path.
3. Add quality assertions that installation uses SetupAPI interface-registry APIs and never parses the symbolic path for a reference string.
4. Run the focused tests and confirm they fail for the expected missing runtime-interface behavior.

## Task 2: Preserve exact topology and audio alias interface paths

**Files:**
- Modify: `crates/voxveil-windows-audio/src/device_interfaces.rs`
- Modify: `crates/voxveil-windows-audio/src/discovery.rs`

1. Extend `TopologyCandidate` with an optional `audio_interface_path`.
2. When `SetupDiGetDeviceInterfaceAlias` succeeds, resolve the alias path with `SetupDiGetDeviceInterfaceDetailW` from the same device-info set.
3. Extend `RuntimeResolution` and `SystemAudioEndpoint` with the selected topology and audio interface paths.
4. Keep exact-path comparison case-insensitive; never interpret or split the path.
5. Re-run Rust tests.

## Task 3: Make runtime binding primary and INF parsing fallback-only

**Files:**
- Modify: `crates/voxveil-windows-audio/src/binding.rs`
- Modify: `crates/voxveil-windows-audio/src/discovery.rs`

1. For `RuntimeBindingKind::Unique`, classify based on resolved PnP metadata + package availability, not topology-reference count.
2. Preserve old reference-string classification only for `RuntimeBindingKind::None`.
3. Match the runtime-capable extension package by hardware ID; do not require a topology reference.
4. Keep detailed fail-closed behavior for ambiguous runtime bindings.
5. Update detail text so a unique runtime binding no longer reports a missing literal reference string as the blocker.
6. Re-run focused Rust tests.

## Task 4: Add an exact-interface FX registration command

**Files:**
- Modify: `native/windows/apo/VoxveilControlCli.cpp`
- Modify: `native/windows/apo/VoxveilControlCli.vcxproj`

1. Add `attach-effects <binding-instance-id> <topology-interface-path> <audio-interface-path>`.
2. Open each supplied opaque path with `SetupDiOpenDeviceInterfaceW`.
3. Resolve the owning devnode with `SetupDiGetDeviceInterfaceDetailW` + `SetupDiGetDeviceInstanceIdW` and reject a binding mismatch.
4. Open the exact interface registry key with `SetupDiOpenDeviceInterfaceRegKey(KEY_READ|KEY_WRITE)`.
5. Under `FX\0`, append Voxveil's SFX CLSID to `PKEY_CompositeFX_StreamEffectClsid` without deleting existing effects; add required processing modes without deleting existing modes; set `PKEY_FX_Association` only when absent.
6. Add `detach-effects` that removes only Voxveil's CLSID and leaves OEM effects intact.
7. Link `Setupapi.lib` and `Advapi32.lib`.
8. Compile the control CLI with warnings-as-errors.

## Task 5: Carry runtime interface identity through elevation

**Files:**
- Modify: `tauri/app/system_audio.rs`
- Modify: `tauri/app/system_audio_revalidation_tests.rs`
- Modify: `scripts/windows/install-system-audio-component.ps1`
- Modify: `scripts/windows/uninstall-system-audio-component.ps1`

1. Add topology/audio interface paths to the endpoint install descriptor.
2. Accept installable runtime bindings without `topologyReference`; keep fallback descriptors compatible with the old reference-string path.
3. Revalidate endpoint ID, binding devnode, metadata devnode, hardware IDs, and driver INF immediately before elevation work.
4. For runtime bindings, call `voxveil-control.exe attach-effects` using the exact paths; for fallback bindings, retain the legacy generated `AddInterface` flow.
5. Persist the runtime paths and binding instance ID in `install-state.json`.
6. During uninstall, detach runtime FX registration before removing Voxveil-owned driver packages.

## Task 6: Make the runtime extension package independent of reference strings

**Files:**
- Modify: `native/windows/package/VoxveilApoExtension.inf.template`
- Modify: `scripts/windows/new-apo-extension-inf.ps1`
- Modify: `scripts/windows/build-windows.ps1`
- Modify: `README.md`

1. Keep the extension INF responsible for `AddComponent`, but remove the normal runtime path's `.Interfaces`/`AddInterface` FX association.
2. Generate the development extension from hardware ID alone for runtime-bound endpoints.
3. Retain a clearly separated legacy fallback generation path only when a runtime interface binding is unavailable.
4. Update package/readme text: runtime SetupAPI binding is primary; INF reference parsing is fallback; production still requires a correctly signed extension/APO package.

## Task 7: Verify end-to-end and keep the repository workflow-free

**Files:**
- Temporary only: `.github/workflows/runtime-interface-verify.yml` (delete before final branch state)

1. Use a temporary Windows Actions workflow on this feature branch to run `cargo test -p voxveil-windows-audio`, Node quality tests, and compile `VoxveilControlCli.vcxproj`.
2. Configure workflow concurrency with `cancel-in-progress: true`, so any newer verification push cancels the older run automatically.
3. Fix all failures and rerun until green.
4. Delete the temporary workflow.
5. Run/verify the repository's workflow-free quality policy against the final tree.
6. Fast-forward `master` only after the final branch comparison contains no temporary workflow or unrelated changes.
