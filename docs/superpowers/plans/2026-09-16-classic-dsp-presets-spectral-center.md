# Selectable Classic DSP Presets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace destructive full-band center cancellation with a persistent non-AI center-suppression path and let users select Music preservation or Balanced behavior in the UI.

**Architecture:** Add a shared `ClassicSuppressionProfile` contract with `MusicPreservation` as the default. The Windows relay owns one persistent `SpectralCenterSuppressor` per stream, using one common low-latency STFT geometry so the profile can be hot-switched without rebuilding the stream; presets only change mask floor/protection parameters. The native APO uses the same profile contract but a no-allocation protected multi-band M/S fallback because its AVRT callback cannot host the Rust STFT implementation.

**Tech Stack:** Rust 1.97, Tauri 2, React/TypeScript, WASAPI, Windows APO C++, existing repository test/quality tooling.

**Spec:** `docs/research/non-ai-vocal-suppression.md` and Wayfinder issue #25.

## Global Constraints

- Classic DSP stays local and non-AI.
- `Vocal = 100` is transparent; `Vocal = 0` is maximum safe suppression, never hard center deletion.
- `Music preservation` is the default and protects ambiguous center music more strongly.
- `Balanced` suppresses vocals more deeply but may lose more center instrumentation.
- Both presets preserve low bass, side-only content, mono audibility, and finite output.
- Common STFT geometry: 512-sample FFT, 128-sample hop; profile switching must not restart the stream.
- DSP algorithmic latency must stay <= 25 ms at 48 kHz.
- No steady-state heap allocation in the audio processing loop.

---

### Task 1: Add the preset contract and red UI/transport tests

**Files:**
- Modify: `crates/voxveil-types/src/processing.rs`
- Modify: `crates/voxveil-types/src/lib.rs`
- Modify: `ui/app/useVoxveilState.test.tsx`
- Modify: `ui/lib/tauri.test.ts`
- Modify: `ui/app/App.test.tsx`

**Interfaces:**
- Produces: `ClassicSuppressionProfile::{MusicPreservation, Balanced}` serialized as `music-preservation` / `balanced`.
- Produces: UI state property `classicSuppressionProfile` and Tauri command `set_classic_suppression_profile` in later tasks.

- [ ] **Step 1: Add failing UI tests**

```ts
expect(screen.getByRole('button', { name: /music preservation/i })).toBeInTheDocument();
expect(screen.getByRole('button', { name: /balanced/i })).toBeInTheDocument();
```

Add client/state tests that call the future setter and assert `invoke('set_classic_suppression_profile', { profile: 'balanced' })`.

- [ ] **Step 2: Run tests and verify RED**

Run: `npm test --workspace @voxveil/ui`
Expected: FAIL because the preset state/client/UI do not exist yet.

- [ ] **Step 3: Add the Rust shared enum only**

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClassicSuppressionProfile {
    #[default]
    MusicPreservation,
    Balanced,
}
```

Re-export it from `voxveil-types`.

- [ ] **Step 4: Run Rust type tests**

Run: `cargo test -p voxveil-types`
Expected: PASS, including serde/default tests for both stable wire values.

- [ ] **Step 5: Commit**

```bash
git add crates/voxveil-types ui/app/useVoxveilState.test.tsx ui/lib/tauri.test.ts ui/app/App.test.tsx
git commit -m "test(dsp): define selectable classic suppression profiles"
```

### Task 2: Establish accompaniment-preservation RED regressions

**Files:**
- Modify: `crates/voxveil-dsp/src/mid_side.rs`
- Create: `crates/voxveil-dsp/src/spectral_center.rs`
- Modify: `crates/voxveil-dsp/src/lib.rs`

**Interfaces:**
- Produces: `SpectralCenterSuppressor::new(sample_rate, vocal_level, profile)`.
- Produces: `set_vocal_level`, `set_profile`, and `AudioProcessor` implementation.

- [ ] **Step 1: Add a regression against the currently destructive public behavior**

```rust
#[test]
fn maximum_suppression_does_not_erase_centered_music() {
    let mut samples = vec![0.5_f32, 0.5; 256];
    let mut dsp = MidSideSuppressor::new(VocalLevel::new(0.0).unwrap());
    dsp.process_stereo_interleaved(&mut samples);
    assert!(samples.iter().any(|sample| sample.abs() > 0.05));
}
```

Use valid Rust vector construction in the actual test and additionally measure low-band retention, vocal-band attenuation, side transparency, bypass transparency, and finite output with deterministic generated tones.

- [ ] **Step 2: Run DSP tests and verify RED**

Run: `cargo test -p voxveil-dsp`
Expected: FAIL because the existing `MidSideSuppressor` erases perfectly centered content at zero vocals.

- [ ] **Step 3: Add spectral processor fixture tests before implementation**

Test these observable thresholds at 48 kHz:

```text
Music preservation, Vocal=0: centered 90 Hz loss <= 3 dB.
Balanced, Vocal=0: centered sustained 1 kHz center component attenuation >= 8 dB.
Side-only signal change < 0.5 dB.
Vocal=100: max absolute sample error <= 1e-6.
Mono signal: remains audible at Vocal=0 for both profiles.
Both profiles: all outputs finite.
latency_frames() <= 1200 at 48 kHz.
Balanced suppresses the ambiguous center band more strongly than Music preservation.
```

- [ ] **Step 4: Commit the RED tests**

```bash
git add crates/voxveil-dsp
git commit -m "test(dsp): guard accompaniment during vocal suppression"
```

### Task 3: Implement persistent adaptive spectral center suppression

**Files:**
- Create: `crates/voxveil-dsp/src/spectral_center.rs`
- Modify: `crates/voxveil-dsp/src/lib.rs`

**Interfaces:**
- Consumes: `VocalLevel`, `ClassicSuppressionProfile`, `AudioProcessor`.
- Produces: a persistent processor with fixed preallocated FFT/window/overlap/mask buffers.

- [ ] **Step 1: Implement one common STFT geometry**

Use constants:

```rust
const FFT_SIZE: usize = 512;
const HOP_SIZE: usize = 128;
```

Precompute a sqrt-Hann analysis/synthesis window, bit-reversal indices, twiddles, overlap-add buffers, previous-bin magnitudes, and smoothed gains in `new`. Use an in-place radix-2 FFT/IFFT so no new dependency or processing-loop allocation is introduced.

- [ ] **Step 2: Implement center likelihood**

For each L/R bin compute bounded balance and phase-coherence cues:

```rust
let balance = 1.0 - (mag_l - mag_r).abs() / (mag_l + mag_r + EPSILON);
let coherence = ((l.re * r.re + l.im * r.im) / (mag_l * mag_r + EPSILON)).clamp(0.0, 1.0);
let center_likelihood = (balance * coherence).clamp(0.0, 1.0);
```

- [ ] **Step 3: Apply profile-specific protection and soft floors**

Use deterministic initial tuning:

```text
Music preservation: floor -12 dB; strong protection below 180 Hz; suppression tapers above 6 kHz; stronger transient protection.
Balanced: floor -18 dB; strong protection below 120 Hz; suppression tapers above 9 kHz; moderate transient protection.
```

Map Vocal 0–100 non-linearly to suppression depth, with exactly unity gain at 100. Estimate spectral flux from the stored previous mid magnitude and reduce suppression for positive transient flux. Smooth bin gains with separate fast attack / slower release coefficients.

- [ ] **Step 4: Keep stereo side energy intact**

Transform each bin to spectral M/S, attenuate only the likely-center part of `M`, then reconstruct L/R. Clamp neither ordinary finite samples nor the side channel; only replace non-finite generated values with zero as a fail-safe.

- [ ] **Step 5: Run DSP tests and verify GREEN**

Run: `cargo test -p voxveil-dsp`
Expected: PASS for low-band preservation, >=8 dB Balanced vocal-band attenuation, side transparency, bypass transparency, mono audibility, finite output, and latency.

- [ ] **Step 6: Commit**

```bash
git add crates/voxveil-dsp
git commit -m "feat(dsp): add adaptive spectral center suppressor"
```

### Task 4: Make the Windows relay own persistent DSP state

**Files:**
- Modify: `crates/voxveil-windows-audio/src/sample.rs`
- Modify: `crates/voxveil-windows-audio/src/relay_engine.rs`
- Modify: `crates/voxveil-windows-audio/src/wasapi_relay.rs`
- Modify: `crates/voxveil-windows-audio/src/relay.rs`
- Modify: `crates/voxveil-windows-audio/src/lib.rs`

**Interfaces:**
- Consumes: `SpectralCenterSuppressor` and `ClassicSuppressionProfile`.
- Produces: `RelayCommand::SetSuppressionProfile`, `RelayHandle::set_suppression_profile`, backend setter, and packet decoding that accepts an existing processor.

- [ ] **Step 1: Write failing relay tests**

Add tests proving a supplied processor survives multiple packet calls and that profile/vocal commands update the same running worker state rather than rebuilding per packet.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p voxveil-windows-audio`
Expected: FAIL because packet processing currently constructs `MidSideSuppressor` on every call and no profile command exists.

- [ ] **Step 3: Refactor packet processing**

Expose:

```rust
pub(crate) fn process_f32le_stereo(
    bytes: &mut [u8],
    processor: &mut dyn AudioProcessor,
) -> Result<(), String>
```

Decode the packet into an already allocated worker buffer, process all stereo frames with the retained processor, then encode back without constructing DSP state inside the packet helper.

- [ ] **Step 4: Retain DSP in the WASAPI worker**

Create one `SpectralCenterSuppressor` after reading the stream sample rate. Route `SetVocalLevel` and `SetSuppressionProfile` commands to its setters. Do not restart capture/render clients when either setting changes.

- [ ] **Step 5: Thread profile through `WindowsAudioBackend`**

Store `classic_suppression_profile`, default to Music preservation, pass it at relay start, and expose a hot setter.

- [ ] **Step 6: Verify GREEN and commit**

Run: `cargo test -p voxveil-windows-audio`
Expected: PASS.

```bash
git add crates/voxveil-windows-audio
git commit -m "feat(windows): retain classic DSP across relay packets"
```

### Task 5: Give the native APO the same safe preset semantics

**Files:**
- Modify: `native/windows/apo/VoxveilSharedState.h`
- Modify: `native/windows/apo/VoxveilApo.h`
- Modify: `native/windows/apo/VoxveilApo.cpp`
- Modify: `native/windows/apo/VoxveilControlCli.cpp`
- Modify: `native/windows/apo/VoxveilApoPolicy.h`
- Modify: `native/windows/apo/tests/VoxveilApoPolicyTests.cpp`
- Modify: `crates/voxveil-windows-audio/src/relay.rs`

**Interfaces:**
- Consumes: stable profile values `music-preservation` and `balanced` from the Rust/UI contract.
- Produces: shared-state profile flag and CLI `profile <music-preservation|balanced>` command.

- [ ] **Step 1: Add failing native policy tests**

Assert both profiles always have a non-zero center floor, Music preservation uses a higher floor / narrower suppression band, and `vocal=100` returns unity.

- [ ] **Step 2: Verify RED on Windows build test target**

Run the APO policy test project through the existing Windows build path.
Expected: FAIL until profile policy helpers exist.

- [ ] **Step 3: Replace full-band hard M/S gain with protected no-allocation fallback**

Retain two per-channel mid filter states inside `CVoxveilApo`. Protect low fundamentals and high air outside a profile-specific vocal band, attenuate only the band-limited mid by the bounded profile floor, and reset filter state at stream lock/unlock boundaries. No heap allocation, locks, file I/O, or blocking calls may enter `APOProcess`.

- [ ] **Step 4: Synchronize profile from Rust backend to APO control**

Whenever Voxveil hands processing to a loaded APO or the user switches presets, run `profile music-preservation` or `profile balanced` before/while enabled.

- [ ] **Step 5: Verify and commit**

Run: `npm run build:windows`
Expected: native APO policy tests and package build PASS.

```bash
git add native/windows/apo crates/voxveil-windows-audio/src/relay.rs
git commit -m "feat(windows): add safe classic presets to APO"
```

### Task 6: Wire preset selection through Tauri and React

**Files:**
- Modify: `tauri/app/state.rs`
- Modify: `tauri/app/dto.rs`
- Modify: `tauri/app/commands.rs`
- Modify: `tauri/platform/controller.rs`
- Modify: `tauri/lib.rs`
- Modify: `ui/lib/types.ts`
- Modify: `ui/lib/demo-state.ts`
- Modify: `ui/lib/tauri.ts`
- Modify: `ui/app/useVoxveilState.ts`
- Modify: `ui/features/engine/EngineScreen.tsx`
- Modify: `locales/en/common.json`
- Modify: `locales/es/common.json`
- Modify: `locales/fr/common.json`
- Modify: `locales/ja/common.json`
- Modify: `locales/ko/common.json`
- Modify: `locales/vi/common.json`
- Modify: `locales/zh/common.json`

**Interfaces:**
- Produces Tauri command: `set_classic_suppression_profile(profile: ClassicSuppressionProfile)`.
- Produces UI setter: `model.setClassicSuppressionProfile(profile)`.

- [ ] **Step 1: Add state/DTO default**

Default `classic_suppression_profile` to `MusicPreservation` and expose it as `classicSuppressionProfile` in the camelCase UI DTO.

- [ ] **Step 2: Add command/controller/client wiring**

The Tauri command must hot-update the Windows backend first; only commit app state after backend update succeeds.

- [ ] **Step 3: Add Engine UI**

Under Classic DSP controls, render an accessible two-option segmented control:

```text
Music preservation — Protects instruments more; some vocal may remain.
Balanced — Stronger vocal reduction; may affect centered instruments more.
```

Show it for the DSP-capable engine path and preserve optimistic rollback behavior already used by `useVoxveilState`.

- [ ] **Step 4: Add translations in every supported locale**

Use the same keys in `en`, `es`, `fr`, `ja`, `ko`, `vi`, and `zh` so `quality:i18n` remains green.

- [ ] **Step 5: Verify GREEN**

Run:

```bash
npm run test --workspace @voxveil/ui
npm run typecheck
npm run quality:i18n
cargo test -p voxveil-tauri
```

Expected: all preset UI, client, command/state, and localization tests PASS.

- [ ] **Step 6: Commit**

```bash
git add tauri ui locales
git commit -m "feat(ui): let users choose classic suppression priority"
```

### Task 7: Update specs and run the full verification gate

**Files:**
- Modify: `docs/research/non-ai-vocal-suppression.md`
- Modify: `docs/specs/audio/classic-dsp.md`
- Modify: this plan checklist as completed.

**Interfaces:**
- Documents exact shipped preset semantics and measured regression thresholds.

- [ ] **Step 1: Record final implementation parameters and fallback behavior**

Document the common 512/128 STFT geometry, Music preservation/Balanced tuning, APO protected-band fallback, and the fact that 0% vocals is bounded suppression rather than isolation.

- [ ] **Step 2: Run full repository verification**

Run:

```bash
cargo test --workspace
npm test
npm run typecheck
npm run quality
npm run build:windows
```

Expected: all commands PASS with no new warnings attributable to this change.

- [ ] **Step 3: Inspect PR diff and status**

Confirm only intended files changed, no generated/binary artifacts were committed, and PR #24 remains mergeable.

- [ ] **Step 4: Commit documentation**

```bash
git add docs
git commit -m "docs(dsp): specify classic suppression presets"
```
