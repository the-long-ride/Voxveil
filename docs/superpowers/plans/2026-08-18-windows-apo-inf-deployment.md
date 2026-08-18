# Windows APO INF Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Install the Voxveil endpoint APO through Windows componentized APO/Extension INF packages instead of protected MMDevices registry writes, while preserving the single-EXE UX.

**Architecture:** A static APO INF installs/registers the user-mode COM APO component in DriverStore. The elevated installer discovers actual enabled render-device hardware IDs and topology interface reference strings, generates a machine-specific Extension INF, and installs both packages with PnPUtil so Windows Endpoint Builder creates the EFX association. The app embeds all static payloads and returns full PnP/SetupAPI diagnostics.

**Tech Stack:** Rust/Tauri, PowerShell 5.1, Windows PnPUtil/SetupAPI, Windows APO/WDK C++, React/Vitest.

**Spec:** `docs/superpowers/specs/2026-08-18-windows-apo-inf-deployment-design.md`

## Global Constraints

- No direct writes to `MMDevices\Audio\Render\*\FxProperties`.
- No ACL/takeown/SYSTEM workaround for protected audio registry keys.
- No `bcdedit`, TESTSIGNING, or Secure Boot changes.
- One downloadable `voxveil.exe`; package files are embedded and staged internally.
- Preserve existing structured UI diagnostics and automatic error-detail dialog.
- Permanent `.github/workflows` remains empty/absent.

---

### Task 1: Lock the INF package contract

**Files:**
- Create: `scripts/ci/check-windows-apo-inf.mjs`
- Modify: `scripts/ci/check-apo-package.mjs`

**Interfaces:**
- Produces: a static contract proving `VoxveilApo.inf` exists, uses the `AudioProcessingObject` class, registers the Voxveil CLSID with HKR, and install code contains no direct MMDevices FxProperties mutation.

- [ ] Write failing Node assertions for `VoxveilApo.inf`, `AddComponent`, `PKEY_CompositeFX_EndpointEffectClsid`, PnPUtil installation, and absence of direct `MMDevices` endpoint mutation.
- [ ] Run the focused Node test and verify it fails because the new INF/generator does not exist yet.
- [ ] Keep the test as the release packaging contract.

### Task 2: Add the componentized APO INF

**Files:**
- Create: `native/windows/apo/VoxveilApo.inf`
- Test: `scripts/ci/check-windows-apo-inf.mjs`

**Interfaces:**
- Consumes: `VoxveilApo.dll`, CLSID `{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}`.
- Produces: PnP model `SWC\VEN_VOXV&CID_APO` with component-relative COM and AudioEngine registration.

- [ ] Add a Windows 11 `Class=AudioProcessingObject` INF patterned on Microsoft's `ComponentizedApoSample.inx`.
- [ ] Copy `VoxveilApo.dll` to DriverStore destination 13.
- [ ] Register COM and AudioEngine APO values under HKR.
- [ ] Run the contract and InfVerif where available.

### Task 3: Discover render targets and generate an Extension INF

**Files:**
- Create: `native/windows/apo/targets.ps1`
- Create: `native/windows/apo/extension.ps1`
- Create: `scripts/ci/check-windows-apo-targeting.mjs`

**Interfaces:**
- Produces: target objects `{ instanceId, hardwareId, topologyRefs[] }` and `New-VoxveilExtensionInf -Targets ... -Path ...`.

- [ ] Write failing fixture tests for parsing PnPUtil render/topology output and generated INF text.
- [ ] Implement PnPUtil block parsing.
- [ ] Enumerate enabled `KSCATEGORY_RENDER` interfaces and matching `KSCATEGORY_TOPOLOGY` interfaces.
- [ ] Resolve the first hardware ID for each render device with `pnputil /enum-devices /instanceid ... /ids`.
- [ ] Recover exact topology reference strings from interface paths when present; preserve empty reference strings when absent.
- [ ] Generate one Extension-INF model/install section per unique render hardware ID.
- [ ] Emit `AddComponent=VoxveilApo`, exact topology `AddInterface` entries, `PKEY_FX_Association`, composite EFX CLSID pid 15, and DEFAULT processing mode.
- [ ] Fail closed if no usable render target/topology interface is discovered.

### Task 4: Replace direct registry installation with PnP installation

**Files:**
- Modify: `native/windows/apo/install.ps1`
- Modify: `native/windows/apo/uninstall.ps1`
- Test: `scripts/ci/check-windows-apo-inf.mjs`

**Interfaces:**
- Consumes: static APO INF + generated Extension INF.
- Produces: `%ProgramData%\Voxveil\apo-installed.json` containing target metadata and OEM INF package names.

- [ ] Remove `Attach-Endpoints`, direct COM/APO HKLM registration, and endpoint backups from the active install path.
- [ ] Preserve UAC elevation and native COM checker.
- [ ] Install generated Extension INF and static APO INF with `pnputil /add-driver ... /install`.
- [ ] Capture stdout/stderr and detect nonzero exit codes.
- [ ] Record installed OEM INF names by comparing `pnputil /enum-drivers` snapshots before/after and by matching Voxveil provider/INF names.
- [ ] On failure append the tail of `%WINDIR%\INF\setupapi.dev.log` and discovered target data to installer details.
- [ ] Update uninstall to delete recorded Voxveil OEM packages through PnPUtil and stop touching endpoint FxProperties.
- [ ] Preserve and restore the development-only `DisableProtectedAudioDG` state.

### Task 5: Embed the INF deployment payload in the standalone EXE

**Files:**
- Modify: `tauri/app/system_audio.rs`
- Modify: `tauri/build.rs`
- Test: existing Rust `system_audio` tests + package contract.

**Interfaces:**
- Produces: embedded `VoxveilApo.inf`, `targets.ps1`, `extension.ps1`, `install.ps1`, `uninstall.ps1` alongside DLL/checker in the temporary package.

- [ ] Extend embedded-payload validation to require the static INF and helper scripts.
- [ ] Stage all INF deployment files into the temporary package.
- [ ] Add `cargo:rerun-if-changed` entries.
- [ ] Keep structured installer error serialization unchanged.
- [ ] Run Rust formatting/tests and the package contract.

### Task 6: Verify on a disposable Windows workflow and produce the EXE

**Files:**
- Temporary only on disposable branch: `.github/workflows/verify-apo-inf.yml`

**Interfaces:**
- Produces: one `voxveil.exe` artifact.

- [ ] Create a disposable branch from the feature head and add a PR-only Windows verification workflow.
- [ ] Run Node package/target-generation tests and PowerShell parser tests.
- [ ] Run `InfVerif` against static INF and a fixture-generated Extension INF when available on the runner.
- [ ] Run UI tests/typecheck, rustfmt, Rust tests, native DSP tests, APO DLL build, and native COM activation.
- [ ] Build `VOXVEIL_EDITION=pro-system` standalone `voxveil.exe`.
- [ ] Run `voxveil.exe --verify-embedded-system-audio`.
- [ ] Upload only the standalone EXE artifact.
- [ ] Download the artifact, verify PE architecture and SHA-256 independently.
- [ ] Remove/close disposable CI plumbing so `feat/windows-apo-component` remains workflow-free.
