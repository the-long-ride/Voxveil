# Windows APO Component Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship and build a usable Windows endpoint APO so Voxveil can process the physical output directly and enable Processing after the component is installed.

**Architecture:** A small native C++ COM/APO DLL performs the zero-latency stereo mid/side suppression in the Windows audio engine. Rust controls it through a ProgramData control file and falls back to the existing WASAPI relay when the APO is absent. PowerShell installation registers the APO and attaches it to render endpoints; GitHub Actions stages the whole system-audio package beside the portable EXE.

**Tech Stack:** C++20/MSVC + Windows SDK APO interfaces, Rust 1.97.1, Tauri 2, PowerShell, GitHub Actions, Node test runner.

**Spec:** `docs/superpowers/specs/2026-08-18-windows-apo-component-design.md`

## Global Constraints

- Windows 10/11 x64 development target.
- APO real-time callback performs no allocation, blocking I/O, registry/file access, or locks.
- Failure is pass-through, never silence.
- Existing WASAPI relay remains fallback.
- Development installer must disclose and restore the `DisableProtectedAudioDG` change.
- Every mutated endpoint registry value must be backed up for uninstall.

---

### Task 1: Native DSP core and APO DLL

**Files:**
- Create: `native/windows/apo/CMakeLists.txt`
- Create: `native/windows/apo/voxveil_dsp.h`
- Create: `native/windows/apo/voxveil_dsp.cpp`
- Create: `native/windows/apo/voxveil_apo.h`
- Create: `native/windows/apo/voxveil_apo.cpp`
- Create: `native/windows/apo/dllmain.cpp`
- Create: `native/windows/apo/tests.cpp`

**Interfaces:**
- Produces `VoxveilApo.dll` with CLSID `{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}`.
- Produces pure `voxveil_process_stereo(float*, uint32_t, uint32_t, bool, uint8_t)` used by APO and tests.

- [ ] Write native DSP tests for disabled pass-through, 100% center suppression, 0% pass-through, and multichannel preservation.
- [ ] Build tests on a Windows Actions runner and confirm the initial target fails before implementation.
- [ ] Implement DSP core.
- [ ] Implement COM class factory and base APO interfaces.
- [ ] Implement format negotiation/lock state and real-time processing.
- [ ] Implement non-RT control-file polling worker with atomics.
- [ ] Build DLL and tests with MSVC; run tests.

### Task 2: Installer and uninstaller

**Files:**
- Create: `native/windows/apo/install.ps1`
- Create: `native/windows/apo/uninstall.ps1`
- Create: `native/windows/apo/README.txt`
- Create: `scripts/ci/check-apo-package.test.mjs`

**Interfaces:**
- Installer accepts `-PackageRoot <path>` and defaults to its own directory.
- Writes `%ProgramData%\Voxveil\apo-installed.json` and `apo-control.bin`.

- [ ] Add Node static tests asserting CLSID consistency, backup/restore behavior markers, protected-audio disclosure, and expected package files.
- [ ] Confirm static tests fail before scripts exist.
- [ ] Implement elevated install script: copy DLL, register COM/AudioEngine entries, backup and attach render endpoints, create control/marker, restart AudioEndpointBuilder/AudioSrv.
- [ ] Implement uninstall script restoring endpoint/protected-audio state and unregistering/removing files.
- [ ] Run static tests green.

### Task 3: Rust APO detection and control

**Files:**
- Modify: `crates/voxveil-windows-audio/src/apo.rs`
- Test: same module

**Interfaces:**
- `ApoBackend::probe()` uses marker/DLL/control presence.
- `set_enabled(bool, u8)` and `set_vocal_level(u8)` atomically rewrite the 3-byte control payload.

- [ ] Replace stub tests with filesystem-backed readiness/control tests using an injectable root.
- [ ] Run Rust tests and confirm they fail against stub implementation.
- [ ] Implement control path helpers and atomic temp-file rename.
- [ ] Implement real probe/set methods while preserving relay fallback.
- [ ] Run crate tests green.

### Task 4: Integrated installer command and UI action

**Files:**
- Modify: `tauri/app/commands.rs`
- Modify: `tauri/lib.rs`
- Modify relevant UI processing/home feature files discovered during implementation.
- Modify UI tests.

**Interfaces:**
- Tauri command `install_windows_audio_component() -> Result<(), String>` locates sibling `system-audio/install.ps1`, launches elevated PowerShell, waits for result, and returns actionable errors.

- [ ] Add command/unit/UI tests for installer action visibility and invocation.
- [ ] Run tests red.
- [ ] Implement Windows command; non-Windows returns unsupported.
- [ ] Add `Install system audio component` button for `ComponentRequired` readiness and reprobe after success.
- [ ] Run UI/Rust tests green.

### Task 5: Build and artifact packaging

**Files:**
- Modify: `.github/workflows/manual-build.yml`
- Modify: `scripts/release/collect-artifacts.mjs`
- Modify: `scripts/release/collect-artifacts.test.mjs`

**Interfaces:**
- Windows build stages `target/system-audio/VoxveilApo.dll`, install/uninstall scripts, README.
- Artifact contains portable EXE and entire `system-audio` directory.

- [ ] Add collector test requiring system-audio package for Windows pro-system.
- [ ] Run collector test red.
- [ ] Add CMake configure/build/test step on Windows.
- [ ] Stage native package before Tauri build.
- [ ] Extend collector and manifest/checksum generation recursively.
- [ ] Run collector test green.

### Task 6: End-to-end CI artifact verification

**Files:**
- Update workflow only if failures reveal integration defects.

- [ ] Trigger Windows/pro-system build from the feature branch.
- [ ] Fix compile/test/package failures at their root cause, one at a time.
- [ ] Verify Windows job succeeds with all native/Rust/UI/artifact checks.
- [ ] Download artifact.
- [ ] Verify ZIP contains `voxveil.exe`, `system-audio/VoxveilApo.dll`, install/uninstall scripts, checksum and manifest.
- [ ] Verify PE architecture and SHA-256 locally.
