# Windows Componentized APO Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the SysVAD/loopback workaround with a real Windows componentized APO package that processes the selected active render endpoint, exposes runtime controls, and can be built manually without GitHub Actions.

**Architecture:** `VoxveilApo.dll` is an in-process SFX APO loaded by AudioDG and performs the existing mid/side vocal suppression on the audio engine real-time thread. `VoxveilControl.dll` plus a tiny control CLI share enable/vocal-level state with the APO; the Rust backend probes installed/attached state and drives the control CLI without unsafe Rust FFI. Installation is device-specific: a generated Extension INF associates the APO with the selected render device/topology interface while a separate AudioProcessingObject INF installs the APO software component.

**Tech Stack:** C++20, ATL/Windows Audio Engine APO APIs, Windows Driver Kit, INF/PnPUtil, PowerShell, Rust/Tauri.

**Spec:** Approved Voxveil Windows runtime-APO design from 2026-08-19.

## Global Constraints

- Target Windows x64 first.
- No fake virtual-output/loopback backend may report `Ready`.
- `Ready` requires installed APO package plus endpoint attachment evidence.
- Install/extension registration targets the selected active render device only; no broad KSCATEGORY_AUDIO writes.
- Keep the repository free of GitHub Actions workflows.
- Manual build produces the app plus native APO/control package in one staged directory.
- Do not claim a production-ready downloadable EXE until the Windows/WDK build and endpoint load are validated end-to-end.

---

### Task 1: Correct Windows readiness semantics

**Files:**
- Modify: `crates/voxveil-windows-audio/src/device.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`
- Replace: `crates/voxveil-windows-audio/src/relay.rs`

**Interfaces:**
- Produces: `WindowsAudioBackend::probe`, `set_enabled`, `set_vocal_level`, `physical_outputs` using componentized APO state rather than a virtual relay.

- [ ] Add tests proving a virtual/SysVAD endpoint alone is never `Ready`.
- [ ] Add a component-state probe contract with explicit `ComponentRequired`/`Ready` states.
- [ ] Replace the WASAPI relay worker with control-command invocation and component-state probing.
- [ ] Run Rust unit tests on Windows and non-Windows targets where available.

### Task 2: Add native real-time APO and control channel

**Files:**
- Create: `native/windows/apo/VoxveilApo.h`
- Create: `native/windows/apo/VoxveilApo.cpp`
- Create: `native/windows/apo/dllmain.cpp`
- Create: `native/windows/apo/VoxveilControl.cpp`
- Create: `native/windows/apo/VoxveilControlCli.cpp`
- Create: `native/windows/apo/VoxveilApo.def`
- Create: `native/windows/apo/VoxveilControl.def`
- Create: `native/windows/apo/VoxveilApo.vcxproj`
- Create: `native/windows/apo/VoxveilControl.vcxproj`
- Create: `native/windows/apo/VoxveilControlCli.vcxproj`
- Create: `native/windows/apo/VoxveilNative.sln`

**Interfaces:**
- Produces: APO CLSID `{F3F2A99F-8FB7-4B88-949E-448BF8A05221}`.
- Produces: C ABI `VoxveilSetEnabled`, `VoxveilSetVocalLevel`, `VoxveilGetState`.
- Shared state name: `Local\\VoxveilApoControl-v1`.

- [ ] Implement an ATL `CBaseAudioProcessingObject` SFX APO with stereo float mid/side suppression matching Rust DSP semantics.
- [ ] Keep `APOProcess` allocation-free, lock-free, and non-blocking.
- [ ] Implement a small named shared-memory control block containing ABI version, enabled flag, vocal percent, and load heartbeat.
- [ ] Export DLL COM entry points and APO registration properties.
- [ ] Add CLI wrapper used by safe Rust to set/query control state.
- [ ] Build x64 Release with VS/WDK and verify DLL exports.

### Task 3: Add componentized APO packaging and precise endpoint attachment

**Files:**
- Create: `native/windows/package/VoxveilApo.inx`
- Create: `native/windows/package/VoxveilApoExtension.inx.template`
- Create: `scripts/windows/new-apo-extension-inf.ps1`
- Replace: `scripts/windows/install-system-audio-component.ps1`
- Replace: `scripts/windows/uninstall-system-audio-component.ps1`

**Interfaces:**
- `new-apo-extension-inf.ps1` resolves the selected/default render endpoint's PnP instance and emits a device-specific Extension INF.
- Install script uses PnPUtil and records the installed OEM INF names for clean uninstall.

- [ ] Define `Class=AudioProcessingObject` software-component INF registering the APO CLSID and `AudioEngine\\AudioProcessingObjects` metadata.
- [ ] Define Extension INF with `AddComponent` and endpoint `FX\\0` association for the Voxveil SFX CLSID.
- [ ] Generate only one target hardware/compatible ID/interface association from the selected active render device.
- [ ] Install package with `pnputil /add-driver ... /install` and fail if attachment cannot be verified.
- [ ] Uninstall only Voxveil-owned OEM INF packages.

### Task 4: Add workflow-free manual Windows build/staging

**Files:**
- Create: `scripts/windows/build-windows.ps1`
- Update: `package.json`
- Update: `README.md`

**Interfaces:**
- Produces: `dist/windows-x64/Voxveil/voxveil.exe`, `system-audio/VoxveilApo.dll`, `VoxveilControl.dll`, `voxveil-control.exe`, INF files, install/uninstall scripts, and SHA-256 manifest.

- [ ] Detect VS/WDK/MSBuild and fail with actionable diagnostics.
- [ ] Build native x64 Release projects, then Tauri `--no-bundle`.
- [ ] Stage all runtime/install files and generate checksums.
- [ ] Add `npm run build:windows` convenience command.
- [ ] Document TESTSIGNING/dev certificate limitations and production signing requirement.

### Task 5: Remove GitHub Actions and verify repository state

**Files:**
- Delete: `.github/workflows/ci.yml`
- Delete: `.github/workflows/manual-build.yml`
- Delete: `.github/workflows/release.yml`
- Delete: `.github/workflows/windows-portable.yml`

- [ ] Delete every workflow file.
- [ ] Confirm `.github/workflows` is empty/absent.
- [ ] Run formatting/tests/build checks available in the execution environment.
- [ ] Do not publish a binary until a Windows machine proves AudioDG loads `VoxveilApo.dll` and audio changes when controls change.
