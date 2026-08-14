# Voxveil Product Requirements Specification

## Product

Voxveil is a local-first real-time vocal reduction application for Windows, Linux, macOS, Android, and iOS.

## Editions

### Standard
Uses normal OS permissions and public APIs.

### Pro System
Uses privileged/system components where required for true system-wide processing, including restricted mobile platforms.

## Functional Requirements

- 100% local/offline audio processing.
- Classic DSP engine must be usable without AI.
- Optional pretrained AI engine only when model/code licensing is commercially safe.
- No model training requirement.
- User-adjustable vocal level/suppression.
- User-adjustable Latency ↔ Quality.
- Auto, Classic DSP, and AI engine selection.
- All Output and Per-App processing.
- Per-app processing toggle and optional setting overrides.
- Calls/VoIP excluded by default.
- Master toggle.
- Desktop hotkeys/tray/menu controls.
- Mobile quick controls where platform support permits.
- Physical output and virtual processed output.
- Optional simultaneous physical + virtual output.
- v1 two-stem scope: vocals + accompaniment.
- Internal processing contracts remain stem-agnostic.

## Failure Policy

Audio continuity outranks separation quality. Degrade from quality AI → balanced AI → fast AI/hybrid → Classic DSP → bypass rather than produce repeated underruns or stop playback.

## Optional AI Model Acquisition

- AI weights are excluded from installers and release artifacts by default.
- Voxveil must not download an AI model automatically.
- A user must explicitly accept a model/license/network notice before download begins.
- Approved models are downloaded directly into device-local Voxveil application data.
- Downloads must be revision-pinned, size-limited, and SHA-256 verified before installation.
- Users must be able to remove downloaded model data.
- No arbitrary model URL field is exposed to the UI.
