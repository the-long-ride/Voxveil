# Classic DSP Engine Specification

## Purpose

Classic DSP is a first-class Voxveil engine and must remain useful with no AI model installed. It favors continuity, low latency, low power, and predictable licensing over perfect source isolation.

## Current v1 algorithm

The foundation implements mid/side suppression for interleaved stereo audio. For each frame:

```text
mid  = (left + right) / 2
side = (left - right) / 2
reduced_mid = mid * vocalLevel
left'  = reduced_mid + side
right' = reduced_mid - side
```

`vocalLevel = 1` preserves the signal. `vocalLevel = 0` removes fully centered content while preserving pure side information.

This is suppression, not semantic vocal separation. Centered drums, bass, and instruments may also be reduced.

## Realtime constraints

- no allocations during processing;
- no filesystem/network/UI calls;
- finite input must remain finite;
- malformed odd trailing sample must not panic;
- engine switch occurs outside the callback and uses buffered crossfade in the future realtime coordinator.

## Future frequency-selective stage

A later DSP milestone may add STFT/spectral masking only after its implementation or FFT dependency passes the same commercial-license, supply-chain, latency, and LOC gates. The v1 engine does not require it to remain functional.
