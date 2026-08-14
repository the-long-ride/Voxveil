# Windows Audio Platform Specification

## Scope

Windows Standard and Pro System editions share the Rust processing graph. The Windows adapter owns capture, process/session discovery, output-device changes, and the optional virtual processed endpoint.

## Standard edition

The intended normal-install route uses Windows Core Audio/WASAPI capabilities available to an ordinary desktop application. The adapter must enumerate active render sessions, support loopback capture where appropriate, retain stable application identities, and restore routing after failure or exit.

## Pro System edition

Pro System may install a signed virtual audio endpoint/driver when true system-wide insertion or a stable virtual processed output cannot be achieved through ordinary session routing. Driver installation and removal must be explicit and reversible.

## Contract

The platform adapter implements `voxveil_routing::AudioRoutingBackend`; capture feeds interleaved floating-point stereo blocks to the shared realtime boundary. Platform code must not contain DSP or model logic.

## Safety

- no undocumented remote service;
- no driver download at runtime;
- signed driver packages only;
- restore previous default/routing state after failures;
- communication sessions bypass processing by default;
- never hold an audio callback on UI, disk, or network work.

## Verification

Tests require session enumeration, output-loss recovery, process-exit handling, format conversion, all-output/per-app switching, and physical-plus-virtual fan-out. Real device tests run on a Windows hardware runner before release.

## Foundation status

The shared contracts and Windows capability module exist. Native WASAPI/virtual-driver implementation is intentionally a later platform milestone and is not represented as complete in this foundation.
