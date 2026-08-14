# Voxveil System Architecture Specification

## Repository Layout

- `/ui`: React + TypeScript presentation.
- `/tauri`: Tauri application integration with no nested `src` directory.
- `/crates`: reusable Rust domain crates.
- `/locales`: local translation resources.
- `/docs/specs`: canonical software specifications.

## Boundaries

Frontend never performs DSP. Rust owns routing, realtime processing, configuration, and platform adapters. UI communicates through narrow typed commands/events.

## Rust Crates

- `voxveil-types`: shared domain types.
- `voxveil-audio-core`: buffer/frame and processing contracts.
- `voxveil-dsp`: classic DSP implementation.
- `voxveil-routing`: global/per-app routing policy.
- `voxveil-model-api`: AI separation abstraction only; no model bundled by default.

## Realtime Rule

Realtime callbacks must avoid network, disk, UI IPC, blocking locks, model loading, and uncontrolled allocation.
