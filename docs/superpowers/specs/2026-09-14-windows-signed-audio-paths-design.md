# Windows Signed Audio Paths Design

Date: 2026-09-14
Status: Approved architecture; implementation pending
Branch: `feat/windows-signed-audio-paths`
Base: `master`

## Goal

Make Voxveil system-wide Windows vocal reduction usable on normal end-user Windows without requiring TESTSIGNING, while preserving the existing componentized APO work and creating a concrete path to first-party Microsoft-signed components.

The design has three tiers:

1. use an already-installed VB-CABLE endpoint as the near-term production interception path;
2. prepare a first-party SysVAD-derived virtual endpoint for Microsoft attestation signing and release ingestion;
3. retain and modernize the existing componentized APO as the long-term native path.

The VB-CABLE and future Voxveil virtual-endpoint tiers share the Rust relay/DSP graph. The APO remains an in-process AudioDG effect with its existing native processing implementation, but all strategies share the same user-facing readiness and control semantics.

## Existing repository state

`master` no longer uses the old SysVAD relay as the current production architecture. The repository currently contains:

- `crates/voxveil-windows-audio` for Windows endpoint discovery/control;
- a componentized SFX APO under `native/windows/apo`;
- `VoxveilApo.inf` plus a generated extension-INF path;
- `scripts/windows/build-windows.ps1` for workflow-free local Windows staging;
- UI/backend flows centered on installing and probing the APO;
- stale development-relay documentation that still describes the former test-signed SysVAD path.

The existing APO work must not be discarded or silently replaced.

## Architecture

### Backend selection

Introduce an explicit Windows interception strategy instead of treating the APO as the only backend.

```text
WindowsInterceptionBackend
  ├─ Apo          existing native componentized APO
  ├─ VbCableRelay near-term signed third-party virtual endpoint
  └─ VoxveilCable future first-party Microsoft-signed virtual endpoint
```

Runtime selection rules:

1. If an installed Voxveil APO is actually loaded by AudioDG on the active physical endpoint, keep using it. This preserves working existing installations and avoids unnecessary virtual routing.
2. Otherwise, if a supported VB-CABLE render endpoint is installed, use the VB-CABLE relay path.
3. Later, if a production Microsoft-signed Voxveil virtual endpoint is installed, it may replace VB-CABLE as the preferred virtual relay backend.
4. Never report `ready` merely because an endpoint exists. `ready` means the selected interception strategy is active and processing audio.

New-user onboarding prioritizes VB-CABLE until the first-party signed virtual endpoint is available.

### Shared relay data flow

For VB-CABLE and the future Voxveil virtual endpoint:

```text
Windows applications
  -> controlled virtual render endpoint
  -> WASAPI loopback capture of that render endpoint
  -> Voxveil shared Rust DSP / optional AI
  -> WASAPI render to selected physical output
```

The virtual render endpoint must not also be selected as the relay's physical output.

## Tier 1: VB-CABLE production relay

### Dependency policy

Voxveil does not bundle, redistribute, silently download, or install VB-CABLE.

The app may:

- detect whether VB-CABLE is installed;
- explain that it is an optional third-party dependency for system-wide Windows processing;
- open the official VB-Audio download page after an explicit user action;
- re-probe after the user completes installation/reboot.

The official VB-Audio documentation identifies the standard endpoints as `CABLE Input` (playback/render) and `CABLE Output` (recording/capture). The relay uses WASAPI loopback on the render endpoint, so it does not depend on selecting the recording endpoint.

### Detection

Extend Windows endpoint discovery with a virtual-endpoint classification layer.

`VbCableEndpoint` should contain at minimum:

- render endpoint ID;
- friendly name;
- active/default state;
- detection confidence/source.

Detection must fail closed. The standard friendly name is a valid initial signal, but the implementation should use stable device metadata where available and avoid accepting arbitrary endpoints that merely contain the word `CABLE`.

Supported initial product: the standard VB-CABLE endpoint. Additional VB-CABLE A/B/C/D variants are out of scope until explicitly tested.

### Physical output

Persist a physical output endpoint ID separately from its display name.

Candidate physical outputs:

- must be active render endpoints;
- must not be the selected virtual interception endpoint;
- must not be a known Voxveil/VB-CABLE virtual endpoint;
- default to the previously selected physical endpoint when still present;
- otherwise fall back to the current non-virtual Windows default when safe.

If no safe physical output exists, the relay does not start.

### Routing state

Voxveil will not use undocumented Windows `PolicyConfig` interfaces in the first implementation to mutate the system default device.

Tier 1 is initially an all-output path. Voxveil considers the cable routed only when the detected VB-CABLE render endpoint is the current Windows default render endpoint used for ordinary playback. Per-application routing does not satisfy this first milestone. Communication-role bypass may remain on the physical device when supported by the existing policy.

States:

- VB-CABLE missing -> `component-required`;
- VB-CABLE present but not the required default render target -> `routing-required`;
- VB-CABLE is the controlled render target, but no safe physical output is available -> `routing-required`;
- relay startup/capture/render fails -> `faulted`;
- relay capture + DSP + physical render are running -> `ready`.

For the first production version, the UI instructs the user to select `CABLE Input` as the Windows output/default device and provides an action to open Windows Sound settings.

### Relay lifecycle

Create a relay service owned by the Windows backend. It must:

1. resolve the virtual render endpoint by endpoint ID;
2. resolve the physical output by endpoint ID;
3. reject input/output identity collisions;
4. initialize WASAPI loopback capture on the virtual render endpoint;
5. initialize WASAPI shared-mode render on the physical output;
6. convert formats only where required;
7. feed blocks through the existing Rust DSP processor;
8. expose live vocal-level updates without rebuilding the audio graph;
9. stop cleanly on disable/app exit;
10. re-probe and recover after device removal/default-device changes.

Do not perform UI, file, network, or blocking process work from real-time audio callbacks.

### Failure behavior

The virtual route intentionally prevents the original system stream from reaching the physical output directly. Therefore relay failure can produce silence.

Required mitigation:

- fail startup before claiming `ready`;
- surface a clear fault immediately;
- stop the relay on unrecoverable endpoint failure;
- preserve the user's remembered physical output selection;
- provide one-click access to Windows Sound settings so the user can restore a physical default output;
- never attempt recursive rendering back into VB-CABLE.

Automatic default-device restoration is deferred until Voxveil has a supported, documented routing mechanism or its own signed endpoint installer can safely own that state transition.

### UI

Replace APO-only system-audio wording with strategy-aware status.

When VB-CABLE is absent:

- explain that system-wide Windows processing needs a virtual audio endpoint;
- show `Get VB-CABLE` linking only to the official VB-Audio site;
- keep processing disabled.

When installed but not routed:

- show detected `CABLE Input`;
- show the selected physical output;
- show `Open Sound settings`;
- explain that Windows output must be routed to the cable before Voxveil can intercept it.

When relay is active:

- show the interception backend (`VB-CABLE`);
- show the physical destination;
- allow normal vocal/quality controls.

All new strings go through the existing localization system.

## Tier 2: first-party SysVAD-derived virtual endpoint

### Objective

Reintroduce SysVAD only as the source for a Voxveil-owned virtual render device that can be submitted for Microsoft production signing. It must remain separate from the current APO package and from the VB-CABLE runtime dependency.

### Repository scope

Add a dedicated driver/package area with:

- stable Voxveil hardware/device identity;
- production INF/catalog layout;
- reproducible x64/Arm64 build scripts as supported by the chosen SysVAD subset;
- package validation;
- CAB generation for Hardware Dev Center attestation submission;
- documentation for Partner Center submission;
- release-ingestion tooling for the Microsoft-signed returned package;
- signature verification before release staging.

Private keys, EV certificate material, Partner Center credentials, and downloaded signed artifacts are not committed.

### Signing boundary

Microsoft's current driver-signing documentation states that attestation signing supports Windows 10 Desktop and later desktop driver scenarios, does not make a package Windows Certified, and requires Hardware Dev Center/Partner Center enrollment with an EV certificate associated with the account.

Repository tooling can prepare and validate the submission, but cannot complete identity vetting, certificate purchase, Partner Center enrollment, or Microsoft approval. Those are external release operations.

### Release integration

The normal Windows release must accept only one of:

- a verified Microsoft-signed Voxveil virtual-driver package; or
- the external VB-CABLE path with no bundled virtual driver.

A test-signed Voxveil SysVAD package must never be staged as an end-user production component.

When the first-party endpoint becomes available, backend discovery can expose it as `VoxveilCable` and share the same loopback/DSP/physical-render relay implementation used by VB-CABLE.

## Tier 3: componentized APO modernization

### Preserve current implementation

Keep the current:

- `VoxveilApo.dll`;
- control DLL/CLI;
- endpoint topology/interface discovery;
- extension-INF generation/install tooling;
- AudioDG loaded-instance readiness checks.

Do not route new-user production support through this path until Windows 11 compliance and signing requirements are validated.

### Windows 11 CAPX work

Modernize the APO toward Windows 11 requirements:

- implement `IAudioSystemEffects3`;
- accept `APOInitSystemEffects3` when supplied;
- use CAPX-compatible property/settings mechanisms rather than inventing additional private registry behavior for new Windows 11 functionality;
- keep compatibility with older initialization structures where required for Windows 10;
- add explicit OS/capability checks rather than assuming the interface version.

Microsoft documentation says new APOs shipping on Windows 11 are required to comply with the Windows 11 APO APIs and be validated through HLK. Therefore Voxveil must not interpret successful attestation signing alone as proof that this APO path is Windows 11 production-qualified.

### External validation gate

Before declaring Tier 3 production-ready:

1. confirm the intended ISV/componentized-APO distribution path with Microsoft Hardware Dev Center support;
2. run the applicable Windows 11 HLK APO tests;
3. resolve any CAPX failures;
4. use the signing/certification path Microsoft requires for that package type;
5. validate on real hardware across representative HDAudio, USB, Bluetooth, HDMI/DisplayPort, and docked outputs.

Until that gate passes, Tier 3 remains an advanced/native path rather than the default end-user unblock.

## Backend API changes

The Windows backend should expose strategy-neutral state instead of APO-specific assumptions.

Suggested internal model:

```rust
enum WindowsInterceptionKind {
    Apo,
    VbCableRelay,
    VoxveilCableRelay,
}

struct WindowsAudioRoute {
    interception: WindowsInterceptionKind,
    source_endpoint_id: Option<String>,
    physical_output_endpoint_id: Option<String>,
}
```

The public UI DTO should expose stable strings rather than Rust enum representations.

Existing readiness values remain:

- `ready`;
- `component-required`;
- `routing-required`;
- `faulted`;
- `unsupported`.

Endpoint installation status used by the APO flow remains separate from runtime backend readiness.

## Configuration

Persist only non-secret local configuration:

- selected physical output endpoint ID;
- preferred interception strategy, if the user explicitly overrides automatic selection;
- previous successful virtual endpoint identity for diagnostics.

Do not persist raw audio, device capture, or third-party installer contents.

## Testing

### Unit tests

Add tests for:

- VB-CABLE endpoint classification and false-positive rejection;
- backend selection precedence;
- readiness state transitions;
- virtual/physical endpoint collision rejection;
- physical-output fallback selection;
- relay state machine startup/stop/error transitions;
- sample conversion/buffering paths required by the relay;
- APO behavior remaining unchanged when it is loaded;
- first-party signed-package validation logic;
- Windows 11 APO initialization structure selection.

### Integration tests

On Windows test hosts:

- VB-CABLE installed / absent;
- cable default / not default;
- physical endpoint removal during playback;
- sample-rate mismatch between cable and physical device;
- enable/disable cycles;
- app restart with stale endpoint IDs;
- APO already installed and loaded;
- relay prevents feedback loops;
- output continues through Voxveil DSP with no duplicate unprocessed physical stream.

### Release tests

Before a Windows release using Tier 1:

- Windows 11 current production release;
- Windows 10 desktop only if it remains in Voxveil's supported-OS policy;
- speakers/headphones over common onboard audio;
- USB audio;
- HDMI/DisplayPort audio;
- Bluetooth output where WASAPI format/latency behavior differs.

Tier 2 requires signature verification on the exact release package. Tier 3 additionally requires the Microsoft/HLK validation gate above.

## Documentation updates

During implementation update:

- `README.md` to describe the three Windows interception strategies accurately;
- `docs/specs/platform/windows.md` with production/backend selection rules;
- `docs/specs/platform/windows-dev-relay.md` to mark the old test-signed SysVAD workflow historical/development-only and point to this architecture;
- Windows build/release docs for attestation package preparation and signed-package ingestion.

Do not restore removed GitHub Actions merely to reproduce the old SysVAD workflow. The repository remains workflow-free unless that policy is changed separately.

## Workstream decomposition and implementation sequence

The three tiers are separate implementation workstreams under one Windows interception architecture. They land sequentially so each workstream can be tested independently:

### Workstream A — Tier 1 runtime unblock

1. Refactor Windows backend selection so APO and relay strategies can coexist.
2. Implement VB-CABLE detection, physical-output selection, and readiness states.
3. Implement the WASAPI loopback -> DSP -> physical render relay.
4. Add UI onboarding/routing guidance and localization.
5. Add Tier 1 Windows integration tests and documentation.

### Workstream B — Tier 2 first-party signed cable preparation

1. Add the SysVAD-derived virtual-driver source/package boundary.
2. Add deterministic build, package validation, and attestation CAB generation.
3. Add Microsoft-signed package ingestion and signature verification.
4. Add `VoxveilCableRelay` discovery using the Tier 1 relay implementation.

### Workstream C — Tier 3 native APO modernization

1. Add `IAudioSystemEffects3` and `APOInitSystemEffects3` handling.
2. Move new Windows 11-facing behavior onto CAPX-compatible mechanisms.
3. Add compatibility and HLK-oriented tests/documentation.
4. Keep production enablement gated on Hardware Dev Center guidance and required HLK/certification results.

Each workstream must keep existing non-Windows builds working and must not claim `ready` until its interception path is actually active.

## Non-goals

This change does not:

- automate certificate purchase or identity vetting;
- store Partner Center credentials;
- redistribute VB-CABLE;
- silently change Windows default audio devices through undocumented APIs;
- claim Windows certification before Microsoft-required validation completes;
- implement per-application Windows routing;
- change the DSP algorithms themselves.

## References

- Microsoft Learn, Driver Code Signing Requirements: https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs
- Microsoft Learn, Driver Signing Options: https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings
- Microsoft Learn, Windows 11 APIs for Audio Processing Objects: https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/windows-11-apis-for-audio-processing-objects
- VB-Audio, VB-CABLE Virtual Audio Device: https://vb-audio.com/Cable/
