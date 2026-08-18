# Windows Manual Binary Artifact Implementation Plan

**Goal:** Make Manual Build publish the raw Windows `voxveil.exe` alongside existing installer packages, then build the `pro-system` Windows artifact from `feat/windows-apo-backend`.

**Architecture:** Keep the existing Tauri build unchanged. Extend artifact collection so Windows builds additionally collect `target/release/voxveil.exe`; keep checksums/manifest generation and the existing upload-artifact step as the single delivery package.

## Tasks

1. Add a failing collector test proving a Windows build includes both an installer and `voxveil.exe`, while non-Windows builds do not collect the raw executable.
2. Update `collect-artifacts.mjs` with a Windows-only raw executable candidate and preserve deterministic metadata generation.
3. Adjust `manual-build.yml` defaults/labels only where useful for direct Windows binary builds; keep `ref`, `platform`, and `edition` dispatch inputs compatible.
4. Run Node quality tests and inspect the workflow diff.
5. Dispatch Manual Build with `ref=feat/windows-apo-backend`, `platform=windows`, `edition=pro-system`.
6. Verify the Windows job succeeds, fetch its `Voxveil-windows-pro-system` artifact, and provide the downloadable ZIP/binary to the user.
