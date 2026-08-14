# Voxveil Security and Supply-Chain Policy

## Objective

Voxveil is local-first and ships without telemetry. Absolute zero supply-chain risk cannot be guaranteed; the repository instead applies strict controls that minimize dependency, build, privilege, and release risk.

## Runtime Security Rules

- No analytics SDK.
- No telemetry SDK.
- No automatic crash upload.
- No general-purpose runtime HTTP capability by default.
- No remote fonts/assets/localization.
- Audio and settings remain local.
- Privileged functionality is behind narrow Rust interfaces.
- Tauri capabilities use least privilege.
- No arbitrary shell execution from the webview.
- No arbitrary filesystem access from the webview.
- No secrets in frontend code.

## Dependency Admission

A dependency is admitted only when all are true:

1. It solves a demonstrated need.
2. License permits free commercial use and redistribution.
3. Exact source/weights/artifact licensing is explicit.
4. It is actively maintained or intentionally vendored/audited.
5. It does not introduce telemetry or undeclared network behavior.
6. Native/install scripts are manually reviewed.
7. Known high/critical vulnerabilities are absent.
8. A small local implementation would not be safer/simpler.

Allowed license family by default:

- MIT
- Apache-2.0
- BSD-2-Clause
- BSD-3-Clause
- ISC
- Unicode-3.0
- Zlib

Any other license requires explicit review and documentation.

## AI Model Rule

Code license and model-weight license are reviewed independently. Do not bundle a checkpoint when commercial use, redistribution, or modification rights are unclear. No NC, research-only, educational-only, or ambiguous community checkpoints.

## Locking and Reproducibility

- Commit `Cargo.lock` and `pnpm-lock.yaml`.
- Use exact toolchain versions in CI.
- Use `--frozen-lockfile` and `--locked`.
- Avoid git dependencies.
- Avoid unpinned binary downloads.
- Pin third-party GitHub Actions to immutable commit SHAs where practical.
- Generate SBOM and dependency-license reports for release artifacts.

## CI Gates

- Rust format/lint/test/coverage.
- Frontend typecheck/test/coverage.
- LOC gate.
- `cargo deny` license/advisory/source checks.
- package-manager audit.
- dependency-license policy check.
- network/telemetry policy scan.
- release checksum generation.

## Privileged Components

Pro System components are treated as a separate trust boundary:

- Separate module/crate.
- Narrow IPC surface.
- Explicit install/uninstall lifecycle.
- Signed binaries where the platform permits.
- No hidden persistence.
- No network behavior.
- No self-update outside the normal signed updater/release path.
