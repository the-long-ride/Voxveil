# Implementation Progress

## Foundation slice implemented

- Repository and custom `/tauri` layout.
- npm-first workspace configuration with lifecycle scripts disabled.
- LOC, i18n, runtime-network, dependency-license, Cargo-source, Tauri-capability, and workflow policy gates.
- Responsive React UI shell for Home, Apps, Routing, Engine, and Settings.
- Editorial Monochrome light/dark/system theme.
- Bundled English, Vietnamese, Chinese, Korean, Japanese, Spanish, and French locales.
- Local theme/language persistence and typed Tauri command client.
- Shared Rust value types, audio processor contract, single-thread fixed-capacity processing queue, routing policy, and model abstraction.
- Classic DSP mid/side vocal suppression v1.
- Calls/VoIP default bypass policy.
- Auto engine selection and graceful degradation primitives.
- Standard/Pro System edition and platform capability contracts.
- CI, manual ten-variant build matrix, tagged release workflow, SHA-256 artifact metadata, and release SBOM generation.

## Deliberately not claimed complete

- Native Windows WASAPI capture/session routing.
- Linux PipeWire capture/routing.
- macOS Core Audio taps/virtual driver.
- Desktop virtual audio drivers.
- Android MediaProjection/root routing implementations.
- iOS supported/privileged routing implementations.
- Tray/global-hotkey implementation.
- Frequency-selective/STFT DSP v2.
- Any AI inference backend or model checkpoint.
- Installer signing/notarization credentials.

Those are independent milestones in `implementation-plan.md`; their interfaces are already separated so they do not require redesigning the UI or shared domain model.

## Verification limitation of this source snapshot

The assembly environment has Node.js/npm but cannot reach the npm registry, and it does not contain Rust/Cargo. Therefore JavaScript/Rust lockfiles, dependency installation, UI compilation/Vitest coverage, Rust compilation, and Rust coverage cannot be honestly produced here. Repository-owned static tests and gates are executable without external dependencies and are run before packaging. The second-pass findings and remaining risks are recorded in `code-review-2026-08-14.md`.
