# Windows APO Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce an APO-first Windows backend selector that safely falls back to the existing WASAPI virtual relay.

**Architecture:** `WindowsAudioBackend` becomes a facade over an APO capability backend and the existing relay backend. The first slice adds selection and diagnostics only; it does not pretend an APO is active before a real installed component exists.

**Tech Stack:** Rust, Tauri, WASAPI, Windows APO/CAPX integration boundary.

**Spec:** `docs/superpowers/specs/2026-08-18-windows-apo-backend-design.md`

## Global Constraints

- Existing relay behavior must remain intact.
- Missing APO support must fall back to the relay.
- No UI or Tauri command contract changes in this slice.
- Never report APO as ready without positive capability detection.

---

### Task 1: Backend selection model

**Files:**
- Create: `crates/voxveil-windows-audio/src/backend.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`

**Interfaces:**
- Produces: `WindowsBackendKind::{Apo, Relay}`, `select_backend(apo, relay)`.

- [ ] Write unit tests proving APO wins only when ready and relay is selected otherwise.
- [ ] Run `cargo test -p voxveil-windows-audio backend` and verify RED.
- [ ] Implement the minimal selection model.
- [ ] Run the focused tests and verify GREEN.
- [ ] Commit.

### Task 2: APO capability backend

**Files:**
- Create: `crates/voxveil-windows-audio/src/apo.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`

**Interfaces:**
- Produces: `ApoBackend::new()`, `ApoBackend::probe() -> BackendProbe`.

- [ ] Write a test proving the initial uninstalled state is `ComponentRequired` and never `Ready`.
- [ ] Run the focused test and verify RED.
- [ ] Implement a conservative APO probe placeholder that reports the component requirement with diagnostic detail.
- [ ] Run tests and verify GREEN.
- [ ] Commit.

### Task 3: Facade over APO and relay

**Files:**
- Modify: `crates/voxveil-windows-audio/src/relay.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`

**Interfaces:**
- `WindowsAudioBackend::probe`, `set_enabled`, `set_vocal_level`, and `physical_outputs` remain source-compatible.
- Produces: `WindowsAudioBackend::active_kind()` for diagnostics/tests.

- [ ] Write tests for selection/fallback behavior around pure selection helpers.
- [ ] Verify RED.
- [ ] Rename the current concrete relay to `RelayBackend` internally and add a selector facade.
- [ ] Verify focused tests GREEN.
- [ ] Run `cargo test -p voxveil-windows-audio`.
- [ ] Commit.

### Task 4: Controller non-regression

**Files:**
- Modify only if required: `tauri/platform/controller.rs`

**Interfaces:**
- No public Tauri command changes.

- [ ] Run controller/workspace tests against the new backend facade.
- [ ] Fix only compile or behavior regressions caused by the backend refactor.
- [ ] Run `cargo test --workspace`.
- [ ] Commit any required fix.

### Task 5: Verification

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run `cargo test --workspace`.
- [ ] Compare branch to `master` and confirm only planned files changed.