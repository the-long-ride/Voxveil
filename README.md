# Voxveil

Voxveil is a local-first, cross-platform real-time vocal-reduction application. It is designed around a shared Rust audio core, native platform audio adapters, and a responsive Tauri/React interface.

## Current implementation

This source tree contains the production-oriented foundation milestone:

- responsive Editorial Monochrome UI with light/dark/system themes;
- English, Vietnamese, Chinese, Korean, Japanese, Spanish, and French bundles;
- local-only state and a narrow typed Tauri command bridge;
- Classic DSP mid/side vocal suppression with no AI dependency;
- stem-agnostic optional AI interface with no model bundled;
- global/per-app routing policy and communication-audio bypass rules;
- fixed-capacity audio buffering and runtime degradation primitives;
- Standard/Pro System edition metadata and platform capability contracts;
- dependency, LOC, i18n, network-surface, workflow, coverage, and license policy gates;
- CI, manual multi-variant build, and semantic-tag release workflows.

Native system-audio interception, virtual audio drivers, privileged Android/iOS hooks, and a commercially cleared AI checkpoint are separate platform milestones. Their contracts and specifications are included, but this foundation does not pretend those drivers already exist.


## Privacy and networking

Voxveil processing is designed to work with no network connection. The application has no telemetry, analytics, remote fonts, cloud audio processing, or generic Tauri HTTP capability. Network-dependent developer operations such as package installation, advisory lookup, and release publishing are build-time/repository operations rather than app runtime behavior.
