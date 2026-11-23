# Realtime Audio Pipeline Specification

## Flow

Capture → SPSC realtime transport → processing worker → SPSC realtime transport → physical/virtual playback.

`LocalFixedQueue` is only a fixed-capacity, single-thread processing primitive. It MUST NOT be used as the cross-thread capture/output transport. Native adapters require a bounded single-producer/single-consumer transport with no allocation, filesystem work, IPC, or blocking mutex on the realtime callback.

The Windows WASAPI endpoint relay polls capture and render on the same worker, so its bounded FIFO is not a cross-thread transport. See [Windows relay buffering](../platform/windows.md#relay-buffering) for startup priming, capture-hole handling, drift correction, and overflow behavior.

## Engines

Classic DSP is mandatory. AI is optional. Auto may select among available engines based on hardware, thermal state, battery, underruns, and the user latency/quality preference.

## Vocal Control

For true separated stems: `output = accompaniment + vocals * vocal_gain`.

For Classic DSP, the same user-facing control maps to suppression strength rather than a literal isolated-vocal gain.

## Switching

Engine/model changes warm the new path and crossfade to avoid discontinuities.
