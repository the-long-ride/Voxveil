# Realtime Audio Pipeline Specification

## Flow

Capture → SPSC realtime transport → processing worker → SPSC realtime transport → physical/virtual playback.

`LocalFixedQueue` is only a fixed-capacity, single-thread processing primitive. It MUST NOT be used as the cross-thread capture/output transport. Native adapters require a bounded single-producer/single-consumer transport with no allocation, filesystem work, IPC, or blocking mutex on the realtime callback.

## Engines

Classic DSP is mandatory. AI is optional. Auto may select among available engines based on hardware, thermal state, battery, underruns, and the user latency/quality preference.

## Vocal Control

For true separated stems: `output = accompaniment + vocals * vocal_gain`.

For Classic DSP, the same user-facing control maps to suppression strength rather than a literal isolated-vocal gain.

## Switching

Engine/model changes warm the new path and crossfade to avoid discontinuities.
