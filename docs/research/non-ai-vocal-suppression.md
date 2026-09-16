# Non-AI Vocal Suppression Research

Parent Wayfinder map: #25
Decision ticket: #26

## Summary

The current full-band mid/side attenuator is behaving exactly as implemented, but its assumption is too destructive for production use: lowering `vocalLevel` attenuates every centered source, not only vocals.

**Recommended production direction:** a persistent, frequency-domain stereo-center suppressor using STFT analysis, stereo center-likelihood, frequency weighting, transient protection, temporal smoothing, and a soft attenuation mask with a non-zero safety floor.

**Simpler fallback:** a low-cost multi-band mid/side suppressor that leaves low bass and high-frequency air/transients mostly untouched while attenuating only the vocal-dominant mid bands.

REPET-style repetition separation is worth retaining as future research, but it should not be the first real-time baseline because it relies on repeating accompaniment structure and substantially more temporal context.

## Verified facts

### Voxveil today

- `crates/voxveil-dsp/src/mid_side.rs` computes `mid=(L+R)/2`, multiplies the entire mid channel by `vocalLevel`, and reconstructs stereo. This necessarily attenuates centered bass, drums, keys, guitars, and other mono-compatible content together with centered voice.
- `docs/specs/audio/classic-dsp.md` already documents this limitation and explicitly reserves a future STFT/spectral-masking milestone.
- `crates/voxveil-windows-audio/src/sample.rs` currently creates a `MidSideSuppressor` inside each processing call. A spectral processor needs persistent state across packets/frames, so the relay must own one processor instance for the lifetime of the stream.

### Stereo-position separation

Barry, Lawlor, and Coyle's ADRess work separates stereo sources in the frequency domain using panning/azimuth information and phase cancellation, without model training. Their paper describes short-time FFT analysis and overlap-add resynthesis, and reports a real-time implementation.

Primary source: https://www.dafx.de/paper-archive/details.php?id=jK8kPFnlWXDwy1Kumk3kkQ

The same paper explicitly notes the exact failure mode Voxveil is seeing: voice is commonly panned in the center together with bass guitar and elements of the drum kit. It recommends band limiting as an additional discriminator when multiple sources share the same center position.

### Harmonic/percussive protection

FitzGerald's median-filter HPSS work separates harmonic and percussive energy from a spectrogram using time- and frequency-axis median filters, then applies masks to the original complex spectrogram. The paper reports that soft masks reduce resynthesis artifacts and discusses real-time feasibility.

Primary source: https://www.dafx.de/paper-archive/2010/DAFx10/DerryFitzGerald_DAFx10_P15.pdf

This is useful as a protection cue, not as a standalone vocal separator: percussive/transient bins can be given less center attenuation so drums survive aggressive vocal reduction.

### Repetition-based separation

Rafii and Pardo's REPET method separates repeating background structure from non-repeating foreground using time-frequency masking. That assumption can work well for pop accompaniment, but it requires enough temporal context to estimate repetition and does not directly solve low-latency arbitrary streaming.

Primary publication DOI: https://doi.org/10.1109/TASL.2012.2213249
Author material: https://zafarrafii.com/Documents/Conferences/Rafii-Pardo%20-%20Music-Voice%20Separation%20using%20the%20Similarity%20Matrix%20-%202012%20%28slides%29.pdf

### FFT dependency candidate

RustFFT 6.4.1 is a pure-Rust FFT library. Its current package metadata declares `rust-version = 1.77`, default SIMD features for AVX/SSE/NEON, and `MIT OR Apache-2.0`. Voxveil currently declares Rust 1.97 and the same dual-license policy, so the stated MSRV and license are compatible with the workspace.

Primary source: https://github.com/ejmahler/RustFFT/blob/master/Cargo.toml

## Recommended architecture

### 1. Persistent `SpectralCenterSuppressor`

Create one processor per active audio stream and retain its FFT plans, overlap buffers, window, previous-mask state, and scratch storage. Allocate these at construction/startup, never in the steady-state processing path.

The Windows relay should own this processor and feed packet audio through it. Byte decoding and DSP state should be separated so a processor is not reconstructed per packet or per stereo frame.

### 2. STFT center-likelihood mask

For each frequency bin, estimate how likely the energy is to belong to the stereo center using both:

- **magnitude balance**: similar left/right magnitudes imply a centered pan position;
- **phase coherence**: near-zero inter-channel phase difference strengthens center confidence.

A simple bounded score can be formed from normalized channel-balance and normalized real cross-spectrum terms. The exact exponents and smoothing constants should be benchmarked rather than guessed into production.

### 3. Frequency protection

Do not suppress all center bins equally.

Initial prototype bands to measure, not yet production constants:

- strong protection below roughly 120–180 Hz so bass/kick fundamentals survive;
- strongest center suppression through the speech/vocal-dominant middle spectrum;
- taper suppression toward the upper treble to preserve cymbal air and stereo detail.

This follows the ADRess observation that center position alone cannot distinguish vocals from center-panned bass/drums and requires an additional discriminator.

### 4. Transient protection

Reduce suppression on bins/frames that look strongly percussive or transient. Start with a cheap spectral-flux/onset protection term. If that is insufficient, evaluate short-window median-filter cues inspired by HPSS.

Do not make full HPSS a hard dependency for the first implementation unless the simpler transient cue fails objective fixtures.

### 5. Soft attenuation, never destructive zeroing

The Classic DSP slider is already specified as *suppression strength*, not literal isolated-vocal gain. Therefore `0% vocals` should mean **maximum safe classical suppression**, not mathematically deleting the whole center channel.

Prototype a bounded attenuation floor instead of zero gain. Candidate maximum reductions to evaluate are approximately 12–18 dB in bins with high center/vocal likelihood. This should preserve accompaniment better and avoid the current collapse.

### 6. Temporal smoothing

Smooth the per-bin mask across frames with separate attack/release behavior. This avoids musical-noise artifacts and rapid image movement when center likelihood changes frame-to-frame.

## Prototype parameters to benchmark

Do not freeze these values before measurement:

| Parameter | Candidate A | Candidate B |
|---|---:|---:|
| Sample rate | 48 kHz | 48 kHz |
| FFT size | 512 | 1024 |
| Hop | 128 | 256 |
| Overlap | 75% | 75% |
| Approx. frame span | 10.7 ms | 21.3 ms |
| Maximum center attenuation | 12 dB | 18 dB |

The first production preset should choose the smallest window that meets quality tests while keeping DSP algorithmic latency near or below ~25 ms. End-to-end relay latency must be measured separately because WASAPI buffering adds its own delay.

## Vocal-control mapping proposal

Keep the UI's 0–100 direction unchanged:

- `100`: transparent/bypass-equivalent DSP gain (`0 dB` suppression).
- `50`: moderate center-vocal attenuation with conservative protection.
- `0`: strongest **safe** non-AI suppression, capped by the mask floor rather than hard center cancellation.

Use a perceptual/non-linear transfer curve so the useful range is not compressed into the bottom few slider values.

## Evaluation / regression plan

Before production implementation, establish an automated red loop around accompaniment preservation.

### Synthetic fixtures

1. **Centered vocal-like harmonic stack + centered bass + wide accompaniment**
   - Existing algorithm fails because both centered components disappear together.
   - New algorithm must significantly attenuate the vocal-band center component while retaining most protected low-bass energy.
2. **Centered kick/snare transient + centered sustained vocal-like tone**
   - Transient energy should be less attenuated than sustained center-vocal energy.
3. **Pure side signal**
   - Must remain effectively unchanged.
4. **Mono input**
   - Must remain audible even at maximum suppression; Classic DSP must not collapse the whole track to silence.
5. **Bypass / 100%**
   - Must remain numerically transparent within the defined tolerance.

### Initial acceptance targets for the prototype

These are engineering targets to validate, not claims about final perceptual quality:

- protected low-band accompaniment loss: no worse than ~3 dB in the synthetic mixed-center fixture;
- vocal-like center attenuation at maximum strength: at least ~8 dB in the same fixture;
- side-only energy change: <0.5 dB;
- no NaN/Inf output;
- no steady-state heap allocation inside processing;
- processing compute stays comfortably below real-time on the Windows x64 baseline runner;
- measured DSP latency <=25 ms before platform buffering.

After synthetic gates pass, add a small redistribution-safe evaluation corpus with isolated vocal/accompaniment references and report accompaniment preservation plus vocal attenuation separately. Do not optimize solely for one scalar score.

## Alternatives and trade-offs

### Multi-band M/S only

**Pros:** simplest, very low CPU/latency, easy to test and license.

**Cons:** still cannot distinguish vocals from center-panned guitars/keys/snare inside the attenuated bands. Useful as a fallback or low-power mode, not the quality ceiling.

### REPET-style separation

**Pros:** can preserve repeating accompaniment even when it is centered; does not require learned weights.

**Cons:** depends on repetition structure and longer history, complicates latency, memory bounds, and transitions. Better as a future high-latency/offline mode than the first streaming engine.

### Full ADRess source extraction

**Pros:** purpose-built stereo source separation with real-time precedent.

**Cons:** more expensive than a center-likelihood mask and still cannot uniquely separate multiple sources that share the same pan position. Voxveil only needs to suppress likely center vocals, so a narrower center-mask design is the smaller first step.

## Decision

Proceed toward **adaptive STFT center suppression with frequency and transient protection** as the first production non-AI quality path. Keep **multi-band M/S** as the fallback/low-power design. Do not implement REPET or full ADRess in the first milestone.

Before production code is written, the design must explicitly settle the quality-vs-latency target and the acceptable behavior for mono/near-mono music.