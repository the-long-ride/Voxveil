# Windows Audio Platform Specification

## Scope

Windows Standard and Pro System editions share the Rust processing graph. The Windows platform layer owns audio interception, session discovery, output-device changes, and the optional virtual processed endpoint.

## Interception rule

WASAPI loopback is a capture source, not an output-replacement mechanism. Voxveil must never treat `loopback capture -> DSP -> same physical endpoint` as system-wide processing because the original render stream is still delivered to that endpoint.

A Windows build may report `backendStatus = ready` only when audio is routed through a Voxveil-controlled interception component that can replace, rather than merely monitor, the original output.

## Standard edition

The normal-install target may use Windows Core Audio for session discovery and capture, but processing becomes active only when a supported controlled render path exists. A future signed virtual render endpoint is the preferred general route because applications can render into it while Voxveil processes one stream and sends the result to the selected physical endpoint.

## Pro System edition

Pro System may install a signed endpoint/mode Audio Processing Object or equivalent privileged system component for true insertion into the Windows audio pipeline. Installation and removal must be explicit and reversible.

## Runtime contract

Until the Windows interception component is installed and initialized:

- `backendStatus` is `component-required`;
- the master Processing switch is disabled;
- backend commands reject attempts to enable processing;
- vocal/quality settings may still be edited as future configuration;
- Voxveil never claims that device audio is being modified.

When the component is implemented, the platform adapter feeds interleaved floating-point blocks through the shared processor and updates `backendStatus` to `ready` only after successful startup.

## Safety

- no arbitrary network service;
- no driver download at runtime;
- signed production system components only;
- restore previous routing/default-device state after failure or exit;
- communication sessions bypass processing by default;
- never perform UI, disk, or network work on the real-time callback;
- if interception initialization fails, preserve unmodified device audio and report a non-ready backend.

## Verification

Tests require backend-readiness gating, session enumeration, output-loss recovery, process-exit handling, format conversion, all-output/per-app switching, physical-plus-virtual fan-out, and crash-safe restoration. Real device tests run on a Windows hardware runner before release.

## Current status

The app-side Windows virtual-endpoint relay is implemented for the first all-output development milestone:

- endpoint discovery and readiness probing;
- loopback capture from a controlled virtual render endpoint;
- Classic DSP/bypass selection in the Rust relay;
- physical-device render output;
- live vocal-level control;
- automatic readiness re-probe when the window regains focus;
- all-output-only capability gating.

Windows development builds can recognize the Voxveil virtual endpoint and the compatible Microsoft SysVAD render endpoint used by the GitHub Actions development bundle. The relay now uses WASAPI loopback capture on that render endpoint, so a separate virtual capture/monitor endpoint is not required. See `windows-dev-relay.md`.

The production Voxveil virtual endpoint, signed driver package, persistent background relay, automatic routing restoration, per-app routing, and AI inference remain separate milestones.

## External virtual driver

Windows system-wide processing requires one controlled virtual render endpoint. The Windows portable GitHub Actions workflow now builds and bundles a Microsoft SysVAD-based development component beside the app. The development driver is test-signed and therefore is not suitable for normal end-user Windows. Production distribution must replace it with a separately reviewed and Microsoft production-signed Voxveil driver package.
