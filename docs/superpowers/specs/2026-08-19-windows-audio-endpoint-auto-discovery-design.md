# Automatic Windows Audio Endpoint Discovery and Installation Design

## Goal

Remove `HardwareId` and topology reference-string entry from the normal Voxveil UI. Voxveil should discover Windows render endpoints automatically, resolve the driver identity needed by the componentized APO package, show compatibility/readiness per output, and launch installation for a selected compatible output.

## Scope

This change covers Windows playback/render endpoints only. Capture devices are excluded. The existing componentized APO remains the processing mechanism.

The first implementation targets automatic discovery and safe endpoint-specific installation. It does **not** pretend that an arbitrary unsigned extension INF can be installed on a normal production Windows machine. If Voxveil cannot produce/use a valid signed package for an endpoint, that endpoint is shown as unsupported for installation rather than asking the user for raw driver identifiers.

## Architecture

### 1. Native discovery in `voxveil-windows-audio`

Add a Windows-only discovery module with a platform-neutral result model:

```text
SystemAudioEndpoint
- endpoint_id
- display_name
- adapter_name
- is_default
- pnp_instance_id
- hardware_ids[]
- driver_inf
- topology_reference
- status
- detail
```

`status` is one of:

- `ready` — Voxveil APO is loaded for the endpoint/device.
- `installable` — endpoint identity and topology binding were resolved and the package requirements are available.
- `component-required` — identity resolved but Voxveil is not attached yet.
- `ambiguous` — more than one plausible topology binding exists and Voxveil cannot safely choose one.
- `unsupported` — endpoint/driver shape is not compatible with the current componentized APO installer.

Discovery flow:

1. Use MMDevice/Core Audio to enumerate active render endpoints and determine the default render endpoint.
2. Read endpoint friendly-name/adapter properties and endpoint device instance identity.
3. Traverse the PnP devnode parent relationship to the underlying audio device and obtain hardware IDs and installed-driver INF identity.
4. Enumerate the audio device's enabled KS interfaces. Prefer supported Windows device/interface APIs rather than parsing opaque MMDevice endpoint IDs.
5. Resolve the topology interface/reference for the same device. When the driver exposes multiple candidate topology interfaces, correlate using device ownership plus the endpoint/KS association. If correlation is not unique, return `ambiguous` instead of guessing.
6. Merge current Voxveil control/readiness information into the endpoint list.

The UI never needs the hardware ID/reference string fields. They remain internal installation metadata.

### 2. Tauri API

Add commands:

```text
list_system_audio_endpoints() -> SystemAudioEndpointDto[]
install_system_audio_component(endpoint_id: String) -> InstallResultDto
```

The install command re-runs discovery immediately before elevation and looks up the endpoint by opaque endpoint ID. It does not trust HardwareId/reference-string values supplied by the UI.

This prevents stale or user-modified driver identifiers from being passed into the elevated installer.

### 3. Elevated installer

Change `install-system-audio-component.ps1` so its normal entry point accepts one resolved endpoint descriptor produced by Voxveil, rather than interactive raw parameters.

The elevated process receives the resolved hardware ID/reference string through a temporary JSON descriptor owned by the current user. Before installation it revalidates:

- target device is still present;
- hardware ID still matches;
- topology interface/reference still matches;
- package type is valid for the target.

The script keeps its lower-level `HardwareId`/`ReferenceString` parameters for developer/manual diagnostics, but the app does not expose them.

### 4. UI

Replace the single generic component-required message with a compact Windows System Audio section.

Example:

```text
Windows System Audio

Speakers — Realtek(R) Audio             Default
Component required                       [Install]

LG Monitor — NVIDIA High Definition Audio
Unsupported by current package           —

USB DAC — FiiO
Ready                                     ✓
```

Add `Refresh` and `Install all compatible outputs` only when at least two endpoints are independently installable. The bulk action runs serially and stops/report failures per endpoint rather than hiding partial failure.

An ambiguous endpoint says that Voxveil could not safely resolve the driver topology. It does not ask the user to paste a hardware ID/reference string.

### 5. Package/signing boundary

Automatic *detection* is universal for supported Windows versions. Automatic *installation* is only offered when Voxveil has a package that Windows can legally load for that endpoint.

For development/test-signed packages, the app may expose a clearly labeled developer-only path. Normal users are never instructed to enable TESTSIGNING or disable Secure Boot as part of the standard installation flow.

A future production distribution can feed the same discovery layer into a catalog of properly signed extension packages keyed by compatible hardware IDs. This design does not require UI changes when that catalog is added.

## Error handling

- Device disappears during install: return `device-changed`, refresh list.
- Topology resolution is non-unique: mark `ambiguous`; never choose first-match.
- Driver changes between discovery and elevation: abort and refresh.
- UAC cancelled: return `cancelled`, no backend state mutation.
- Package installs but AudioDG does not load the APO: return `installed-not-loaded` and keep processing disabled.
- One endpoint failing does not corrupt statuses for other endpoints.

## Testing

### Rust unit tests

- endpoint-to-parent devnode mapping;
- deterministic default endpoint marking;
- unique topology candidate becomes resolved;
- multiple topology candidates become `ambiguous`;
- unsupported/missing driver metadata is fail-closed;
- install lookup uses endpoint ID and ignores caller-supplied raw driver values.

### Quality/UI tests

- raw HardwareId/reference-string fields are absent from the user-facing install flow;
- component-required state renders discovered endpoint rows;
- only `installable` endpoints expose Install;
- ambiguous/unsupported endpoints cannot invoke install;
- bulk install is shown only for two or more compatible endpoints.

### Windows build validation

- `cargo test -p voxveil-windows-audio`;
- `cargo test -p voxveil-tauri`;
- `npm run test:quality`;
- full `scripts/windows/build-windows.ps1` build;
- package smoke test confirms `voxveil.exe`, `VoxveilApo.dll`, control binaries, installer and discovery path are staged.

A hosted runner cannot prove real AudioDG attachment to arbitrary physical hardware, so real-device attachment remains a dedicated Windows-machine validation gate.

## Success criteria

1. User opens Voxveil and sees all active Windows playback endpoints without entering IDs.
2. Default playback endpoint is clearly identified.
3. Voxveil internally resolves hardware/driver/topology metadata when it can do so safely.
4. User installs by selecting an endpoint, not by pasting driver strings.
5. Ambiguous or unsupported drivers fail closed with useful status.
6. Processing becomes Ready only after the APO is actually observed as loaded.
7. Repository remains workflow-free after one-shot build validation.