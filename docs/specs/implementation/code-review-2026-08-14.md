# Voxveil Foundation Code Review — 2026-08-14

## Scope

Second-pass review of the foundation source for duplication, fail-safe behavior, architectural boundaries, release safety, and supply-chain policy enforcement.

## Corrected weaknesses

- Centralized repeated screen-intro and navigation rendering into reusable UI components.
- Centralized local-storage access and language persistence instead of duplicating browser storage handling.
- Centralized optimistic UI mutation plus native recovery in `useVoxveilState`.
- Native UI now starts from a fail-safe disabled state and falls back to that state when command recovery cannot reach the backend.
- Native Rust state now starts disabled, physical-output-only, with vocals untouched.
- Communication bypass uses a shared enum rather than string literals across Rust DTO/command code.
- Processing load is a domain enum; the Rust backend no longer emits UI translation keys.
- Virtual/Both output choices are disabled when unavailable and independently rejected by the Rust command boundary.
- Platform capability presets are centralized instead of repeated across platform modules.
- `FixedQueue` was renamed `LocalFixedQueue` and documented as single-thread-only; it is not presented as the future cross-thread realtime transport.
- Shared filesystem walkers replace duplicate traversal logic in quality/release scripts.
- Shared dependency approval rules prevent npm and Cargo commercial-license policy from drifting.
- Cargo policy scans workspace and target-specific dependency sections; npm wildcard workspaces are expanded and audited.
- Release publishing stages unique filenames, preventing duplicate `manifest.json`/`SHA256SUMS` asset collisions.
- Release tags are checked against npm, Cargo, Tauri, and workspace versions.
- Repository hygiene rejects generated compiler/profile artifacts in source snapshots.
- Production network gate covers browser networking primitives and Rust socket primitives.

## Remaining engineering risks

These are intentionally not disguised as completed functionality:

1. **Realtime cross-thread transport** — `LocalFixedQueue` is not thread-safe. Native audio adapters require a bounded SPSC transport that does not allocate or block on the audio callback.
2. **Native audio adapters** — WASAPI, PipeWire, Core Audio, Android, and iOS routing are still capability/interface scaffolding.
3. **Virtual audio endpoints** — no production virtual driver/device exists yet.
4. **AI model integration** — no checkpoint is bundled. Any future model must pass exact code + weight commercial redistribution review.
5. **Dependency lockfiles** — `package-lock.json` and `Cargo.lock` must be generated once on a trusted networked machine and committed before CI/release can install dependencies.
6. **Compiled verification** — this assembly environment lacks Cargo and cannot reach the npm registry, so UI typecheck/Vitest coverage and Rust compile/clippy/coverage remain external verification gates.
7. **Platform build credentials/toolchains** — mobile signing, Apple notarization, and privileged Pro-System packaging require platform-specific CI secrets/toolchains.

## Review rule going forward

Prefer deletion or small local code over a new dependency. Any new dependency, privileged capability, network primitive, unsafe Rust, or realtime callback allocation requires an explicit specification update and quality-gate change before merge.
