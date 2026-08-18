# Windows APO Component Design

## Goal

Make Voxveil system-audio processing usable on Windows without a virtual render endpoint by shipping a real post-mix Audio Processing Object (APO), an endpoint installer, runtime detection/control, and a Windows development artifact that contains everything required to enable it.

## Runtime architecture

`Windows Audio Engine -> Voxveil endpoint effect APO -> physical speaker/headphones`.

The APO is an in-process COM DLL loaded by the Windows audio engine. It implements `IAudioProcessingObject`, `IAudioProcessingObjectRT`, and `IAudioProcessingObjectConfiguration`, accepts one input and one output connection, performs in-place-compatible float processing, and reports zero algorithmic latency. The real-time callback allocates no memory, performs no file/registry access, and calls no blocking APIs.

The initial algorithm mirrors Voxveil's current stereo mid/side suppressor: derive mid and side from L/R; scale the mid component using the user's vocal-level value; reconstruct L/R. For multichannel buffers only channels 0/1 are altered and remaining channels pass through unchanged.

## Control channel

The app writes a compact control file in `%ProgramData%\Voxveil\apo-control.bin` containing a version byte, enabled byte, and vocal-level byte. The APO owns a non-real-time worker that polls the file and updates atomics. `APOProcess` only reads those atomics.

## Installation

The development package ships:

- `system-audio/VoxveilApo.dll`
- `system-audio/install.ps1`
- `system-audio/uninstall.ps1`
- `system-audio/README.txt`

The installer requires elevation, copies the DLL into `%ProgramFiles%\Voxveil\system-audio`, registers the COM class and AudioEngine APO metadata, attaches the APO to active render endpoint `FxProperties`, creates/restores endpoint backups, creates the ProgramData control file, enables system effects, and restarts the Windows Audio service.

For unsigned development builds, the installer explicitly enables `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\DisableProtectedAudioDG=1`. The installer records the prior value and the uninstall script restores it. This is development-only and must be replaced by a signed deployment path before production release.

Endpoint attachment uses the documented endpoint-effect property key (`PKEY_FX_EndpointEffectClsid`) or composite endpoint-effect key when preserving an existing endpoint effect. The installer backs up every changed endpoint value before mutation and restores it on uninstall.

## App behavior

`ApoBackend::probe()` becomes real: it requires the installed marker, DLL, and control file. When present it reports `Ready` and names the physical default output when available. `set_enabled` and `set_vocal_level` update the control file atomically. When APO is not ready, existing relay fallback remains available.

A Windows-only Tauri command launches `system-audio/install.ps1` through PowerShell with UAC elevation. The UI shows an `Install system audio component` action when readiness is `ComponentRequired`, then reprobes after the installer completes.

## Build and artifact

GitHub Actions builds `VoxveilApo.dll` on the Windows runner before the Tauri application, stages the DLL and scripts into `target/system-audio`, and the Windows artifact collector includes the directory beside `voxveil.exe`.

The development binary is considered complete only when CI proves:

1. native APO unit tests pass;
2. APO DLL builds as PE x64;
3. Rust/TypeScript tests pass for probe/control/install command behavior;
4. `voxveil.exe` builds;
5. the artifact contains `voxveil.exe`, `system-audio/VoxveilApo.dll`, install/uninstall scripts, checksums, and manifest.

## Compatibility and safety

- Windows 10/11 x64 development target.
- No virtual output is required for APO mode.
- Existing relay remains fallback when the APO is absent.
- Installation requires Administrator/UAC.
- Endpoint registry values are backed up and restored.
- Protected-audio disabling is development-only, surfaced in installer output, and restored on uninstall.
- APO failure must fail open to pass-through audio rather than silence output.
