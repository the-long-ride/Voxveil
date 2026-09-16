# Non-AI Vocal Suppression Research

Parent Wayfinder map: #25
Decision ticket: #26

## Summary

Voxveil's original full-band mid/side attenuator was too destructive for production use: lowering `vocalLevel` attenuated every centered source, not only vocals.

PR #24 now implements the selected production direction: a persistent, local, non-AI frequency-domain stereo-center suppressor using STFT analysis, stereo center-likelihood, frequency weighting, transient protection, temporal smoothing, and a non-zero attenuation floor.

The native Windows APO uses the selected low-cost fallback: protected band-limited mid suppression with the same Music preservation / Balanced center floors and no heap allocation in the realtime callback.

REPET-style repetition separation remains future research only. It requires substantially more temporal context and is not part of the current real-time baseline.

## Implemented baseline in PR #24

### Persistent Rust relay DSP

`crates/voxveil-dsp/src/spectral_center.rs` owns the Classic DSP implementation used by the Windows relay.

- one `SpectralCenterSuppressor` is retained for the stream lifetime;
- fixed FFT size: `512` frames;
- fixed hop: `128` frames;
- fixed algorithmic latency: `511` frames (~10.65 ms at 48 kHz, ~11.59 ms at 44.1 kHz);
- precomputed sqrt-Hann window, bit-reversal table and FFT twiddles;
- preallocated FFT, overlap-add, previous-magnitude and smoothed-gain state;
- custom in-place radix-2 FFT/IFFT, so RustFFT was not added as a runtime dependency;
- WASAPI `BUFFER_SILENT` packets still advance the retained processor using zero input;
- live vocal/profile changes mutate the retained processor instead of rebuilding the stream.

The original `MidSideSuppressor` behavior remains useful as historical context, but it is no longer the production Windows relay path implemented by this PR.

### Center-likelihood mask

For each L/R frequency bin, the processor estimates center likelihood from both magnitude balance and phase coherence:

```text
balance = 1 - abs(|L| - |R|) / (|L| + |R| + epsilon)
coherence = clamp(real(L * conj(R)) / (|L| * |R| + epsilon), 0, 1)
center_likelihood = clamp(balance * coherence, 0, 1)
```

The spectral mid component is attenuated; the side component is retained. The center score is squared before use to reduce suppression of ambiguous stereo material.

### Implemented profile parameters

| Parameter | Music preservation | Balanced |
|---|---:|---:|
| Minimum center gain | `0.25118864` (-12 dB) | `0.12589255` (-18 dB) |
| Fully protected low band | <= 180 Hz | <= 120 Hz |
| Full suppression weight reached by | 350 Hz | 260 Hz |
| High-band taper begins | 5.5 kHz | 7 kHz |
| Fully protected high band | >= 9 kHz | >= 12 kHz |
| Transient protection factor | `0.75` | `0.45` |

Suppression depth uses `(1 - vocalLevel)^0.72`, with exactly unity suppression depth at `Vocal = 100`.

Per-bin gains use faster attack and slower release smoothing (`0.55` attack / `0.14` release in the current implementation). Positive spectral flux reduces suppression according to the selected profile's transient-protection factor.

`Vocal = 0` means strongest bounded Classic DSP suppression for the selected profile. It does **not** mean mathematical center deletion or semantic vocal isolation.

### Live-state behavior

The processor is intentionally persistent across control changes.

- A live transition to `Vocal = 100` resets retained smoothed spectral gains to unity without rebuilding the processor or clearing overlap-add state.
- A live **Balanced -> Music preservation** transition also resets retained smoothed gains to unity so the protective profile does not temporarily inherit stronger Balanced attenuation.
- A Music preservation -> Balanced transition keeps normal attack smoothing.
- Previous spectral magnitude and overlap-add history remain continuous across those changes.

Behavioral regressions live in:

- `crates/voxveil-dsp/tests/hot_vocal_bypass.rs`;
- `crates/voxveil-dsp/tests/hot_profile_switch.rs`;
- `crates/voxveil-dsp/tests/sample_rate_44100.rs`.

They are committed but still require executable Cargo evidence on the current PR head.

### Native Windows APO fallback

The APO cannot host the relay STFT implementation directly in its realtime callback without violating the branch's realtime constraints. It therefore uses an allocation-free protected mid-band fallback.

`native/windows/apo/VoxveilApoPolicy.h` carries the same profile identities and center floors:

- Music preservation: -12 dB minimum center gain;
- Balanced: -18 dB minimum center gain;
- `vocal=100`: processing bypass / unity behavior;
- profile-specific one-pole low/high band filters approximate the protected vocal band at common Windows 44.1/48 kHz mix rates.

The APO callback does not return to full-band hard center cancellation.

## Research basis

### Stereo-position separation

Barry, Lawlor, and Coyle's ADRess work separates stereo sources in the frequency domain using panning/azimuth information and phase cancellation, without model training. Their paper describes short-time FFT analysis and overlap-add resynthesis and reports a real-time implementation.

Primary source: https://www.dafx.de/paper-archive/details.php?id=jK8kPFnlWXDwy1Kumk3kkQ

The paper also describes the central limitation relevant to Voxveil: voice can share the center position with bass guitar and drum elements. Position alone therefore needs an additional discriminator such as band protection.

### Harmonic/percussive protection

FitzGerald's median-filter HPSS work separates harmonic and percussive energy from a spectrogram using time- and frequency-axis median filters, then applies masks to the original complex spectrogram. Soft masking reduces resynthesis artifacts and the paper discusses real-time feasibility.

Primary source: https://www.dafx.de/paper-archive/2010/DAFx10/DerryFitzGerald_DAFx10_P15.pdf

Voxveil does not implement full HPSS in this milestone. It uses positive spectral flux as the cheaper transient-protection cue so attacks/percussion receive less center attenuation.

### Repetition-based separation

Rafii and Pardo's REPET method separates repeating background structure from non-repeating foreground using time-frequency masking. That assumption can work well for pop accompaniment, but it requires enough temporal context to estimate repetition and does not directly solve low-latency arbitrary streaming.

Primary publication DOI: https://doi.org/10.1109/TASL.2012.2213249
Author material: https://zafarrafii.com/Documents/Conferences/Rafii-Pardo%20-%20Music-Voice%20Separation%20using%20the%20Similarity%20Matrix%20-%202012%20%28slides%29.pdf

## Current evaluation boundary

Synthetic tests are necessary but not sufficient for production tuning.

### Synthetic acceptance targets

The current regression direction remains:

- Music preservation centered 90 Hz loss <= ~3 dB;
- Balanced sustained centered 1 kHz attenuation >= ~8 dB;
- side-only change <0.5 dB;
- mono/near-mono content remains audible at maximum suppression;
- finite input produces finite output;
- `Vocal = 100` is transparent within the committed numerical tolerance;
- DSP algorithmic latency <=25 ms before WASAPI buffering.

### Real-audio acceptance

Use `docs/testing/classic-dsp-fixture-corpus.md` and `docs/testing/classic-dsp-evaluation.md`.

The release matrix requires:

- at least two independent native 44.1 kHz controlled fixtures;
- at least two independent native 48 kHz controlled fixtures;
- both Music preservation and Balanced at `Vocal = 0`;
- an unprocessed reference for every fixture;
- independently licensed natural production mixes for subjective reverb/doubles/mastering/artifact evaluation;
- recorded hashes, provenance/license evidence, residual-vocal assessment, accompaniment damage, stereo/peak/alignment evidence, CPU/dropouts and end-to-end latency.

Downloaded sources, derived fixtures, renders, manifests and measurements stay under ignored `.local-evaluation/classic-dsp/` and are not committed.

No production tuning constant should be changed solely from anecdotal listening; changes should be backed by repeatable fixture evidence.

## Alternatives and trade-offs

### Protected multi-band M/S

**Pros:** lowest CPU/latency, allocation-free, easy to reason about and suitable for the native APO callback.

**Cons:** cannot distinguish vocals from center-panned guitars/keys/snare inside the attenuated band. It is the current APO fallback, not the relay quality ceiling.

### REPET-style separation

**Pros:** can preserve repeating accompaniment even when centered; no learned weights required.

**Cons:** depends on repetition structure and longer history, increasing latency/memory/transition complexity. It remains a possible future high-latency/offline mode.

### Full ADRess source extraction

**Pros:** purpose-built stereo source separation with real-time precedent.

**Cons:** more complex than the current center-likelihood mask and still cannot uniquely separate multiple sources sharing the same pan position.

## Decision

The first production non-AI path is **persistent adaptive STFT center suppression with frequency and transient protection**, with **protected band-limited M/S** as the native APO realtime fallback.

The remaining work is validation and evidence-based tuning, not another algorithm redesign: run the exact-head verification gate, evaluate the approved 44.1/48 kHz corpus, measure Windows CPU/dropouts/end-to-end latency, then adjust constants only if the evidence supports it.
