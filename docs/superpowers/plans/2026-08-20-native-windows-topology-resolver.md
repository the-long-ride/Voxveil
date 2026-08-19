# Native Windows Topology Resolver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Voxveil's INF-first Windows playback binding resolver with a runtime-first native resolver that can safely resolve OEM audio endpoints such as Realtek XU.

**Architecture:** Keep existing Core Audio playback enumeration, but resolve each endpoint natively through `IMMDevice`/`IDeviceTopology`, then correlate the connected adapter with present `KSCATEGORY_TOPOLOGY` device interfaces through SetupAPI. Preserve the PowerShell INF parser strictly as fallback/reference-string recovery for the already-resolved device. Browser DTOs remain opaque and installation revalidates the same binding before elevation.

**Tech Stack:** Rust, `windows` 0.62.2, Windows Core Audio DeviceTopology API, SetupAPI, existing `wasapi` 0.23.0, Tauri, PowerShell fallback, Vitest.

**Spec:** `docs/superpowers/specs/2026-08-20-native-windows-topology-resolver-design.md`

## Global Constraints

- Runtime Windows APIs are the primary source of truth; INF parsing is fallback only.
- Never parse a topology reference string from a device-interface symbolic link.
- Never guess between multiple plausible topology bindings.
- Hardware IDs, devnode IDs, interface paths, and topology references remain native-only.
- A unique runtime binding with no matching signed package is `ComponentRequired`, not `Unsupported`.
- Installation must re-run resolution and abort if the device/driver/binding changed.
- Voxveil repository must end with only `master` and no permanent GitHub Actions workflows.

---

### Task 1: Pure runtime candidate correlation model

**Files:**
- Create: `crates/voxveil-windows-audio/src/device_interfaces.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`
- Test: unit tests in `device_interfaces.rs`

**Interfaces:**
- Produces `TopologyCandidate { device_instance_id: String, interface_path: String, alias_match: bool }`.
- Produces `select_topology_candidate(adapter_device_id: &str, candidates: &[TopologyCandidate]) -> CandidateSelection` where `CandidateSelection` is `Unique(TopologyCandidate)`, `Ambiguous`, or `None`.

- [ ] **Step 1: Write RED tests** covering exact adapter/devnode match, alias-preferred match, two indistinguishable candidates => ambiguous, and unrelated candidates => none.
- [ ] **Step 2: Run** `cargo test -p voxveil-windows-audio device_interfaces -- --nocapture` and verify failures are caused by the missing model/selector.
- [ ] **Step 3: Implement the minimal pure selector** with case-insensitive Windows device-ID comparison and no heuristic string parsing.
- [ ] **Step 4: Re-run the focused tests** and verify all Task 1 tests pass.
- [ ] **Step 5: Commit** `feat: add topology candidate correlation model`.

### Task 2: Native DeviceTopology adapter identity

**Files:**
- Create: `crates/voxveil-windows-audio/src/topology.rs`
- Modify: `crates/voxveil-windows-audio/Cargo.toml`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`
- Test: unit tests around result classification plus Windows-only integration test in `topology.rs`

**Interfaces:**
- Add target Windows dependency `windows = { version = "=0.62.2", features = ["Win32_Foundation", "Win32_Media_Audio", "Win32_System_Com"] }`.
- Produce `resolve_adapter_device_id(endpoint_id: &str) -> Result<Option<String>, String>`.

- [ ] **Step 1: Write a RED Windows-only test** that empty/unknown endpoint IDs return structured `Ok(None)`/`Err` without panicking and add a pure helper test for normalizing device IDs.
- [ ] **Step 2: Run** `cargo test -p voxveil-windows-audio topology -- --nocapture` on Windows and verify the new tests fail before implementation.
- [ ] **Step 3: Implement COM initialization + MMDevice lookup** using `MMDeviceEnumerator`/`IMMDeviceEnumerator::GetDevice` for the existing endpoint ID.
- [ ] **Step 4: Activate `IDeviceTopology`**, inspect connectors, use `IConnector::GetConnectedTo` / `GetDeviceIdConnectedTo` and `IDeviceTopology::GetDeviceId` to obtain the connected hardware adapter identity. Treat inactive/unconnected paths as `Ok(None)` so fallback remains available.
- [ ] **Step 5: Run Task 2 tests** and verify they pass on Windows.
- [ ] **Step 6: Commit** `feat: resolve playback adapter through DeviceTopology`.

### Task 3: Enumerate active KSCATEGORY_TOPOLOGY interfaces with SetupAPI

**Files:**
- Modify: `crates/voxveil-windows-audio/Cargo.toml`
- Modify: `crates/voxveil-windows-audio/src/device_interfaces.rs`
- Test: Windows-only integration tests in `device_interfaces.rs`

**Interfaces:**
- Extend `windows` features with `Win32_Devices_DeviceAndDriverInstallation` and `Win32_System_Registry` only if required by generated SetupAPI signatures.
- Produce `enumerate_topology_interfaces() -> Result<Vec<TopologyCandidate>, String>`.
- Use constant GUID `{DDA54A40-1E4C-11D1-A050-405705C10000}`.

- [ ] **Step 1: Add a RED Windows test** that enumeration returns `Ok(Vec<_>)` and never panics even on a runner with no physical audio endpoint.
- [ ] **Step 2: Run** the focused Windows test and confirm failure because enumeration is not implemented.
- [ ] **Step 3: Implement `SetupDiGetClassDevsW` + `SetupDiEnumDeviceInterfaces` + `SetupDiGetDeviceInterfaceDetailW`**, collecting each interface path and owning `SP_DEVINFO_DATA` instance ID.
- [ ] **Step 4: Add `SetupDiGetDeviceInterfaceAlias` correlation support** so candidates can be flagged `alias_match=true` when Windows confirms an alias relationship; do not derive a reference string from the path.
- [ ] **Step 5: Re-run Task 3 tests** and verify pass.
- [ ] **Step 6: Commit** `feat: enumerate runtime topology interfaces`.

### Task 4: Runtime-first discovery orchestration with INF fallback

**Files:**
- Modify: `crates/voxveil-windows-audio/src/discovery.rs`
- Modify: `scripts/windows/discover-system-audio-endpoints.ps1`
- Test: unit tests in `discovery.rs`; existing PowerShell empty-input smoke test

**Interfaces:**
- Runtime resolution path: endpoint ID -> adapter device ID -> topology candidates -> unique binding.
- PowerShell helper becomes `recover_reference_for_device` fallback for the already-selected device only; it may not retarget a different parent/device.
- Preserve `SystemAudioEndpoint` public shape; optional diagnostic source may be internal-only unless UI needs it.

- [ ] **Step 1: Write RED tests** for: runtime unique binding beats empty INF; multiple runtime candidates => `Ambiguous`; zero runtime candidates invokes fallback; runtime-selected device cannot be replaced by another INF match; unique runtime binding + missing package => `ComponentRequired`.
- [ ] **Step 2: Run** `cargo test -p voxveil-windows-audio discovery -- --nocapture` and verify expected RED failures.
- [ ] **Step 3: Refactor `enrich_endpoints`** to call native runtime resolver first and invoke PowerShell only when runtime cannot recover the literal reference string for the same device.
- [ ] **Step 4: Narrow the PowerShell contract** to accept the already-resolved device identity and return reference candidates only for that device; retain current full-PnP fallback only when DeviceTopology itself cannot resolve an adapter.
- [ ] **Step 5: Re-run Windows audio tests** with `cargo test -p voxveil-windows-audio` and the PowerShell `[]` smoke test.
- [ ] **Step 6: Commit** `fix: prefer runtime topology resolution over INF parsing`.

### Task 5: Installation revalidation and Realtek-facing diagnostics

**Files:**
- Modify: `crates/voxveil-tauri/src/lib.rs` or current install-command module containing endpoint install logic
- Modify: `ui/features/home/SystemAudioEndpoints.tsx` only if copy/status mapping needs adjustment
- Test: existing Tauri endpoint tests and UI endpoint tests

**Interfaces:**
- `install_system_audio_endpoint(endpoint_id)` must re-run the runtime resolver before writing the elevated descriptor.
- If runtime binding differs from the displayed binding, abort with a refresh-required error.

- [ ] **Step 1: Write RED Tauri test** proving install aborts if re-resolution returns changed/ambiguous binding.
- [ ] **Step 2: Run** `cargo test -p voxveil-tauri` and verify the new case fails for the intended reason.
- [ ] **Step 3: Implement binding revalidation** before UAC descriptor creation.
- [ ] **Step 4: If necessary, update endpoint detail copy** so runtime-resolved/no-package reads as signed component required rather than generic unavailable.
- [ ] **Step 5: Run** `cargo test -p voxveil-tauri` and `npm run test --workspace @voxveil/ui`.
- [ ] **Step 6: Commit** `fix: revalidate runtime audio binding before install`.

### Task 6: Full verification and Windows package

**Files:**
- Temporary build workflow in disposable GitGrab branch only; no permanent workflow in Voxveil.
- Build output: `dist/windows-x64/Voxveil/`

**Interfaces:**
- Build exact Voxveil `master` commit.
- Upload artifact `Voxveil-windows-x64-master-native-topology`.

- [ ] **Step 1: Verify source tree** has only `master` and no `.github/workflows` in Voxveil.
- [ ] **Step 2: Run full checks on Windows:**
  - `cargo test -p voxveil-windows-audio`
  - `cargo test -p voxveil-tauri`
  - `npm run test --workspace @voxveil/ui`
  - `npm run typecheck`
  - `npm run test:quality`
- [ ] **Step 3: Run** `scripts/windows/build-windows.ps1` and require success.
- [ ] **Step 4: Smoke-test package:** required EXE/DLL/scripts exist, `voxveil-control.exe status` succeeds, discovery handles empty input, `voxveil.exe` launches without a nonzero immediate exit.
- [ ] **Step 5: Upload/download ZIP**, verify SHA-256 and required contents.
- [ ] **Step 6: Delete disposable build branch/workflow** and verify Voxveil remains master-only/workflow-free.
- [ ] **Step 7: Hand the user the new ZIP** for Realtek XU validation.
