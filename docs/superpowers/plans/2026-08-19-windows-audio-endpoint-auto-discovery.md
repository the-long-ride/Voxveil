# Windows Audio Endpoint Auto-Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Discover Windows playback endpoints automatically, resolve safe device/topology binding metadata internally, and let users install Voxveil per endpoint without entering Hardware IDs or reference strings.

**Architecture:** Keep Core Audio enumeration in the safe `voxveil-windows-audio` crate using the existing pinned `wasapi` wrapper. Add a Windows-only resolver that enriches each Core Audio endpoint by invoking a bundled PowerShell discovery helper that uses Windows PnP/CIM data and the installed driver INF; the resolver fails closed when the endpoint-to-PnP or topology binding is non-unique. Tauri exposes opaque endpoint IDs only; installation re-resolves the endpoint immediately before elevation and passes a generated JSON descriptor to the bundled elevated installer.

**Tech Stack:** Rust 1.97, `wasapi = 0.23.0`, serde/serde_json, Tauri 2, PowerShell 7/Windows PowerShell 5.1-compatible cmdlets, PnP/CIM, React 19, TypeScript 7, Vitest.

**Spec:** `docs/superpowers/specs/2026-08-19-windows-audio-endpoint-auto-discovery-design.md`

## Global Constraints

- Windows playback/render endpoints only; capture devices are out of scope.
- Keep `#![forbid(unsafe_code)]` in `voxveil-windows-audio` and Tauri.
- Never choose the first of multiple plausible PnP/topology matches; mark the endpoint `ambiguous`.
- The UI sends only the opaque endpoint ID to install commands, never HardwareId or ReferenceString.
- Installation re-runs discovery immediately before elevation and aborts on stale/mismatched metadata.
- A normal user is never instructed to enable TESTSIGNING or disable Secure Boot.
- `Ready` still requires observed APO load (`loaded > 0`); successful package staging alone is not Ready.
- Keep the repository workflow-free after one-shot Windows validation.

---

### Task 1: Add endpoint discovery domain model and fail-closed resolver

**Files:**
- Create: `crates/voxveil-windows-audio/src/discovery.rs`
- Modify: `crates/voxveil-windows-audio/src/device.rs`
- Modify: `crates/voxveil-windows-audio/src/relay.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`
- Modify: `crates/voxveil-windows-audio/Cargo.toml`
- Create: `scripts/windows/discover-system-audio-endpoints.ps1`

**Interfaces:**
- Produces: `SystemAudioEndpoint { endpoint_id, display_name, adapter_name, is_default, pnp_instance_id, hardware_ids, driver_inf, topology_reference, status, detail }`.
- Produces: `SystemAudioEndpointStatus::{Ready, Installable, ComponentRequired, Ambiguous, Unsupported}`.
- Produces: `WindowsAudioBackend::system_audio_endpoints() -> Result<Vec<SystemAudioEndpoint>, String>`.
- The PowerShell helper accepts a JSON array of Core Audio `{ endpointId, displayName, isDefault }` descriptors on stdin and emits JSON enrichment records on stdout.

- [ ] **Step 1: Write resolver unit tests** for unique topology selection, multiple topology references becoming `Ambiguous`, missing hardware IDs becoming `Unsupported`, and default endpoint preservation.

```rust
#[test]
fn multiple_topology_candidates_fail_closed() {
    let status = classify_binding(true, &["Topology".into(), "HeadphoneTopology".into()], true);
    assert_eq!(status, SystemAudioEndpointStatus::Ambiguous);
}
```

- [ ] **Step 2: Run `cargo test -p voxveil-windows-audio` on the Windows validation runner** and confirm the new tests fail before implementation.
- [ ] **Step 3: Implement `discovery.rs`** with serde DTOs, deterministic classification helpers, helper-process invocation, and JSON parsing; keep all Rust safe.
- [ ] **Step 4: Implement `discover-system-audio-endpoints.ps1`**. For each Core Audio row, uniquely match a present `AudioEndpoint` PnP device by friendly name; read `DEVPKEY_Device_Parent`, then parent Hardware IDs and driver INF; parse only the matching installed INF's `AddInterface`/string definitions for `KSCATEGORY_TOPOLOGY`. Return `ambiguous` if endpoint/PnP or topology matching is not unique.
- [ ] **Step 5: Integrate with `relay.rs`** so Core Audio enumeration supplies endpoint ID/name/default flags, helper enrichment is merged, and the currently loaded APO state upgrades only the active default endpoint to `Ready` when `loaded > 0`.
- [ ] **Step 6: Run Windows audio tests** and commit.

### Task 2: Expose discovery and opaque-ID installation through Tauri

**Files:**
- Modify: `tauri/app/dto.rs`
- Modify: `tauri/app/commands.rs`
- Modify: `tauri/platform/controller.rs`
- Modify: `tauri/lib.rs`
- Modify: `ui/lib/types.ts`
- Modify: `ui/lib/tauri.ts`

**Interfaces:**
- Produces Tauri command: `list_system_audio_endpoints() -> Result<Vec<SystemAudioEndpointDto>, String>`.
- Replaces install command signature with: `install_system_audio_component(endpoint_id: String) -> Result<InstallResultDto, String>`.
- Produces `InstallResultDto { endpoint_id, outcome, detail }`, outcome values `launched`, `cancelled`, `device-changed`, `installed-not-loaded`.

- [ ] **Step 1: Add Tauri unit tests** proving install lookup accepts only endpoint ID and refuses `Ambiguous`/`Unsupported` endpoints.
- [ ] **Step 2: Run `cargo test -p voxveil-tauri` and confirm RED** before wiring commands.
- [ ] **Step 3: Add controller methods** `system_audio_endpoints()` and endpoint lookup/revalidation delegation.
- [ ] **Step 4: Add DTO conversions and commands**, register them in `tauri/lib.rs`, and change the TypeScript client to call the new command signatures.
- [ ] **Step 5: Run Tauri tests** and commit.

### Task 3: Make the elevated installer descriptor-driven and revalidating

**Files:**
- Modify: `scripts/windows/install-system-audio-component.ps1`
- Modify: `scripts/windows/build-windows.ps1`
- Test: `scripts/quality/check-windows-relay.test.mjs`

**Interfaces:**
- Normal app path: `install-system-audio-component.ps1 -EndpointDescriptor <json-path>`.
- Developer diagnostic path remains: `-HardwareId <id> -ReferenceString <ref> [-TestSign]`.
- Descriptor contains endpoint ID, PnP instance ID, chosen Hardware ID, driver INF and topology reference.

- [ ] **Step 1: Extend quality tests** to require descriptor mode and prohibit raw IDs in the user-facing Tauri/UI path.
- [ ] **Step 2: Run `npm run test:quality` and confirm RED**.
- [ ] **Step 3: Implement descriptor loading/revalidation** using present PnP device data before generating/installing the extension INF. Reject changed hardware IDs, driver INF or topology reference with `device-changed`.
- [ ] **Step 4: Stage `discover-system-audio-endpoints.ps1`** in `system-audio/` from `build-windows.ps1` and update package README copy to describe automatic detection instead of manual ID entry.
- [ ] **Step 5: Run quality tests** and commit.

### Task 4: Add Windows System Audio endpoint UI

**Files:**
- Create: `ui/features/home/SystemAudioEndpoints.tsx`
- Create: `ui/features/home/SystemAudioEndpoints.test.tsx`
- Modify: `ui/features/home/HomeScreen.tsx`
- Modify: `ui/app/useVoxveilState.ts`
- Modify: `ui/theme/components.css`
- Modify: `locales/en/common.json`
- Modify: `locales/vi/common.json`
- Modify: `locales/zh/common.json`
- Modify: `locales/ko/common.json`
- Modify: `locales/ja/common.json`
- Modify: `locales/es/common.json`
- Modify: `locales/fr/common.json`

**Interfaces:**
- Model fields: `systemAudioEndpoints`, `systemAudioEndpointsBusy`, `systemAudioInstallBusyId`, `refreshSystemAudioEndpoints()`, `installSystemAudioEndpoint(endpointId)`, `installAllSystemAudioEndpoints()`.
- Component props use those model fields; no raw binding metadata is rendered.

- [ ] **Step 1: Write component tests**: all endpoint rows render; Default badge renders; only `installable`/`component-required` resolved endpoints offer Install; ambiguous/unsupported rows have no install button; bulk button appears only for 2+ installable endpoints.
- [ ] **Step 2: Run `npm run test --workspace @voxveil/ui` and confirm RED**.
- [ ] **Step 3: Implement the hook state/actions** with refresh on native startup/focus, serial bulk installation, per-endpoint error reporting and post-install refresh.
- [ ] **Step 4: Implement `SystemAudioEndpoints.tsx` and replace the generic install button in Home**.
- [ ] **Step 5: Add compact responsive styling and locale-key parity across all seven languages**.
- [ ] **Step 6: Run UI tests, typecheck, quality/i18n checks** and commit.

### Task 5: Full Windows verification and artifact

**Files:**
- Temporary only: `.github/workflows/maintenance-auto-endpoint-once.yml`
- Delete the temporary workflow after artifact capture.

**Interfaces:**
- Artifact: `Voxveil-windows-x64-master-auto-endpoints.zip` from the exact tested master commit.

- [ ] **Step 1: Add one-shot Windows 2022 workflow** that removes `.github/workflows` from the checked-out tree before quality checks.
- [ ] **Step 2: Run** `cargo test -p voxveil-windows-audio`, `cargo test -p voxveil-tauri`, `npm run test:quality`, `npm run test --workspace @voxveil/ui`, `npm run typecheck`, and `scripts/windows/build-windows.ps1`.
- [ ] **Step 3: Smoke-test staged package**: verify app/APO/control/discovery/installer files, run `voxveil-control status`, and launch `voxveil.exe` for at least 8 seconds.
- [ ] **Step 4: Upload and download the artifact** only when all prior steps succeed.
- [ ] **Step 5: Delete the temporary workflow from master**, verify only `master` remains and `.github/workflows` is absent.
- [ ] **Step 6: Report the real-machine limitation explicitly**: hosted CI can prove enumeration code compiles and package wiring works, but it cannot prove endpoint mapping/AudioDG loading against the user's physical driver until run on that machine.
