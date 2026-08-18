# Windows APO-First Backend Design

## Goal

Prefer direct Windows audio-engine processing through an Audio Processing Object (APO) while retaining the existing virtual-render WASAPI relay as a compatibility fallback.

## Architecture

Voxveil keeps the Rust DSP implementation as the canonical signal-processing implementation. The Windows host selects one of two transport backends:

1. `ApoBackend` — preferred when a Voxveil APO component is installed and attached to the active physical render endpoint.
2. `RelayBackend` — existing `Voxveil Output -> WASAPI loopback -> DSP -> physical render` path.

`WindowsAudioBackend` becomes a selector/facade. Tauri continues to interact only with `WindowsAudioBackend`; no command API changes are required for the first slice.

## Backend selection

Selection is deterministic:

- If APO capability reports `Ready`, select APO.
- Otherwise select relay and expose the relay's readiness status unchanged.
- APO detection must never make a previously working relay unavailable.
- The active backend kind is observable for diagnostics/tests.

## First production-safe slice

This change establishes the abstraction and capability model without shipping an unsigned/incomplete APO binary. `ApoBackend` initially reports `ComponentRequired` unless an installed component can be positively identified in a later task. It must not claim audio processing is active until real endpoint attachment/control exists.

The relay implementation remains behaviorally unchanged.

## Native APO boundary

The future native component is a small Windows APO DLL responsible for COM/APO/CAPX integration and real-time buffer callbacks. Signal processing remains in shared Voxveil DSP code behind a stable C ABI boundary. The native callback must allocate no memory, perform no blocking I/O, and avoid locks on the realtime path.

## Failure behavior

- Missing/unattached APO: fall back to relay.
- APO probe error: record diagnostic detail, fall back to relay.
- Relay unavailable: return its existing `ComponentRequired`, `RoutingRequired`, or `Faulted` state.
- Enabling processing must only succeed when the selected backend is ready.

## Testing

Unit tests cover backend priority, APO-unavailable fallback, and non-regression of relay readiness mapping. Existing relay and DSP tests remain unchanged.

## Scope

This slice does not package/sign/install an APO driver component. It creates the production-safe backend seam required to add that component without another Tauri/UI rewrite.