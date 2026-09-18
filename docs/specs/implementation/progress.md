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
- Classic DSP adaptive STFT stereo-center suppression with user-selectable Music preservation and Balanced profiles.
- Persistent Windows relay DSP state with hot vocal-level/profile updates.
- Native Windows APO real-time-safe band-limited fallback with matching non-zero profile floors.
- Calls/VoIP default bypass policy.
- Auto engine selection and graceful degradation primitives.
- Standard/Pro System edition and platform capability contracts.
- Manual ten-variant build matrix, tagged release workflow, SHA-256 artifact metadata, and release SBOM generation.
- Windows Tier 1 relay, Tier 2 signed virtual-driver staging/lifecycle, and Tier 3 signed APO/CAPX source paths with scoped package ownership, reboot tombstones, post-operation Driver Store verification, and untracked-package rejection.
- APO management is fail-closed to one endpoint-scoped install state at a time until native telemetry can prove loaded instances per endpoint; UI installation remains explicit per endpoint rather than bulk.

## Deliberately not claimed complete

- Production signing/HLK validation for Windows virtual endpoint/APO packages.
- Linux PipeWire capture/routing.
- macOS Core Audio taps/virtual driver.
- Android MediaProjection/root routing implementations.
- iOS supported/privileged routing implementations.
- Tray/global-hotkey implementation.
- Any AI inference backend or model checkpoint.
- Installer signing/notarization credentials.

Those are independent milestones in `implementation-plan.md`; their interfaces are separated so they do not require redesigning the UI or shared domain model.

## Verification policy

The repository intentionally permits only its manual build workflow. Source changes must therefore be verified with repository-owned static/unit/build checks where the execution environment supports them, plus the manual Windows workflow for native Windows packaging. A source snapshot or environment that cannot execute a required toolchain must not be treated as evidence that the corresponding build or test passed.
