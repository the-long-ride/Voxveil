# Voxveil

Voxveil is a local-first, cross-platform real-time vocal-reduction application. It is designed around a shared Rust audio core, native platform audio adapters, and a responsive Tauri/React interface.

## Current implementation

This source tree contains:

- responsive Editorial Monochrome UI with light/dark/system themes;
- English, Vietnamese, Chinese, Korean, Japanese, Spanish, and French bundles;
- local-only state and a narrow typed Tauri command bridge;
- Classic DSP mid/side vocal suppression with no AI dependency;
- stem-agnostic optional AI interface with no model bundled;
- global/per-app routing policy and communication-audio bypass rules;
- fixed-capacity audio buffering and runtime degradation primitives;
- Standard/Pro System edition metadata and platform capability contracts;
- dependency, LOC, i18n, network-surface, coverage, license, repository-hygiene, and workflow-free policy gates;
- a Windows x64 componentized Audio Processing Object implementation under `native/windows/apo`;
- workflow-free local/manual Windows application + APO staging via `npm run build:windows`.

### Windows system audio

The Windows backend no longer treats a SysVAD/virtual render endpoint as the production processing path. `VoxveilApo.dll` is an in-process SFX APO intended to be loaded by Windows AudioDG on the selected physical render endpoint. The UI/backend controls it through `VoxveilControl.dll` and `voxveil-control.exe`; backend readiness requires an actual loaded APO instance rather than merely detecting a virtual endpoint.

The APO uses the Windows componentized-audio package model:

- `native/windows/package/VoxveilApo.inf` installs the APO software component and COM/audio-engine registration;
- `native/windows/package/VoxveilApoExtension.inf.template` associates the APO CLSID with one selected render driver's audio topology interface;
- `scripts/windows/new-apo-extension-inf.ps1` generates the device-specific development Extension INF;
- `scripts/windows/install-system-audio-component.ps1` installs either a locally test-signed development package or a matching prebuilt production-signed package;
- `scripts/windows/uninstall-system-audio-component.ps1` removes only Voxveil-owned driver packages.

For a development install, use a dedicated Windows/WDK machine with TESTSIGNING enabled and pass the selected render device's hardware ID and topology reference string to the installer. Normal Secure Boot/end-user distribution requires production-signed APO/catalog files and a device-compatible prebuilt Extension INF. The repository does not represent an unsigned/test-signed package as production-ready.

Build the Windows development package from an x64 Developer PowerShell with Visual Studio C++ Build Tools and the Windows Driver Kit installed:

```powershell
npm run build:windows
```

The staged output is written to `dist/windows-x64/Voxveil` by default. The manual builder runs the Windows-audio Rust tests and repository quality tests, builds the native control/APO projects, builds the Tauri executable with `--no-bundle`, and writes SHA-256 checksums.

Other privileged native platform hooks and a commercially cleared AI checkpoint remain separate platform milestones.

## Privacy and networking

Voxveil processing is designed to work with no network connection. The application has no telemetry, analytics, remote fonts, cloud audio processing, or generic Tauri HTTP capability. Network-dependent developer operations such as package installation, advisory lookup, and release publishing are build-time/repository operations rather than app runtime behavior.

### Optional AI model

Voxveil does not bundle AI weights. The Engine screen can install a reviewed model only after explicit user consent. Downloads are pinned to an approved source revision, stored under Voxveil's local application-data directory, and SHA-256 verified before installation. The model can be removed from the same screen. See `docs/specs/audio/ai-model-delivery.md`.
