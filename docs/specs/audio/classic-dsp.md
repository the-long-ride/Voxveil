# Classic DSP Engine Specification

## Purpose

Classic DSP is a first-class Voxveil engine and must remain useful with no AI model installed. It favors continuity, low latency, low power, and predictable licensing over perfect source isolation.

## Current algorithm

The Windows relay uses a persistent adaptive STFT stereo-center suppressor. One processor instance is owned for the lifetime of the audio stream so FFT tables, overlap-add buffers, previous-bin magnitudes, gain smoothing, and profile state survive packet boundaries.

Processing geometry is fixed at a 512-frame transform with a 128-frame hop. For each frequency bin the processor estimates center likelihood from left/right magnitude balance and phase coherence, then attenuates the mid component while preserving the side component. The mask also applies:

- low-frequency protection for bass fundamentals;
- high-frequency protection for stereo detail;
- spectral-flux transient protection for percussion/attacks;
- attack/release gain smoothing;
- a non-zero suppression floor so maximum reduction never deletes the center completely.

WASAPI packets flagged `BUFFER_SILENT` are zero-filled and still passed through the retained processor. Silent source periods therefore advance the STFT timeline, overlap-add state, and fixed latency instead of pausing processor state until the next audible packet.

`vocalLevel = 1` preserves the signal. A live transition to `vocalLevel = 1` resets the retained smoothed spectral gains to unity without rebuilding the processor; audio after the existing fixed latency therefore does not inherit suppression release from the previous setting. Lower values increase center suppression inside the profile's protected frequency range.

## User-selectable profiles

### Music preservation

This is the default. It prioritizes keeping centered instruments intact when the processor cannot confidently distinguish them from vocals.

- minimum center gain: `0.25118864` (-12 dB);
- low protection: full below 180 Hz, ramping to full suppression by 350 Hz;
- high protection: suppression starts tapering after 5.5 kHz and is fully protected by 9 kHz;
- stronger transient protection.

Some vocal may remain when the stereo evidence is ambiguous.

### Balanced

This profile trades more center-instrument attenuation for stronger vocal reduction while retaining a non-zero center floor.

- minimum center gain: `0.12589255` (-18 dB);
- low protection: full below 120 Hz, ramping to full suppression by 260 Hz;
- high protection: suppression starts tapering after 7 kHz and is fully protected by 12 kHz;
- lighter transient protection.

The profile can be changed while processing is active; the retained processor updates its mask parameters without restarting the stream. A live **Balanced → Music preservation** transition resets retained smoothed spectral gains to unity without restarting the stream or rebuilding the processor, so the more protective profile does not temporarily inherit stronger Balanced attenuation. A Music preservation → Balanced transition keeps the normal attack smoothing.

## Native APO fallback

The Windows APO callback cannot run the relay STFT implementation directly without violating its real-time constraints. It therefore uses a bounded, allocation-free band-limited mid suppressor with the same two profiles and -12/-18 dB center floors. The APO profile is carried through the shared control state and is re-synchronized during relay-to-APO handoff.

The APO fallback must never return to full-band hard center cancellation.

## Realtime constraints

- no allocations inside the active processing callback/path after stream setup;
- no filesystem/network/UI calls from the processing callback;
- finite input must remain finite;
- malformed odd trailing samples must not panic;
- profile/vocal-level updates must not reconstruct the processor per audio packet;
- silent capture packets must advance the same retained processor state as audible packets;
- engine switching occurs outside the callback and uses the realtime coordinator.

## Quality boundary

Classic DSP is stereo-position/spectral suppression, not semantic source separation. It cannot perfectly distinguish a centered singer from centered instruments in all mixes. The two profiles expose that tradeoff explicitly instead of silently destroying all centered content.

## Validation protocol

Use `docs/testing/classic-dsp-fixture-corpus.md` for the approved fixture provenance/license boundary and `docs/testing/classic-dsp-evaluation.md` for repeatable real-audio A/B evaluation. The repository includes the dependency-free `voxveil-dsp` example `classic_dsp_raw`, which renders raw stereo f32 input with fixed-latency compensation so Music preservation and Balanced outputs stay sample-aligned with the source fixture.

Quantitative tuning must use the controlled corpus matrix with independent native 44.1 kHz and 48 kHz fixtures. Subjective release acceptance must also include independently licensed natural production mixes; controlled mixtures alone are not sufficient evidence for reverb, mastering, doubles, or production-artifact behavior.

Do not commit downloaded evaluation audio. Record fixture/version provenance and license checks, source and raw-input SHA-256 values, deterministic mix recipe, rendered-output hashes, profile/Vocal settings, accompaniment damage, residual vocal, artifacts, CPU, and end-to-end latency before changing production tuning constants. Restricted/non-commercial datasets are not a commercial release baseline without separate permission for the exact recording.
