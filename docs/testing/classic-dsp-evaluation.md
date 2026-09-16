# Classic DSP real-audio evaluation

Use this protocol to tune **Music preservation** and **Balanced** without changing their product semantics.

## Fixture requirements

Use only audio that may legally be stored or evaluated by the tester. Prefer tracks with separately available vocal and accompaniment stems, but do not commit copyrighted evaluation audio to the repository.

Keep a local fixture manifest with:

- fixture ID and source/license;
- genre and approximate arrangement;
- original sample rate;
- whether clean vocal/accompaniment stems exist;
- notes about strongly centered instruments, bass, percussion, stereo ambience, and vocal placement.

The production acceptance pass should include at least:

- sparse vocal + accompaniment;
- dense full-band mix;
- strong centered bass/kick/snare;
- wide stereo ambience/reverb;
- mono or near-mono material;
- male and female lead vocals;
- harmony/double-tracked vocals;
- 44.1 kHz and 48 kHz source material.

## Prepare deterministic input

The current tuning reference rate is 48 kHz stereo float32. Convert a fixture without loudness normalization or other processing:

```powershell
ffmpeg -v error -i .\fixture.wav -map_metadata -1 -ac 2 -ar 48000 -f f32le -y .\fixture-48k.f32
```

Record the SHA-256 of the raw evaluation input so repeated runs use identical bytes:

```powershell
Get-FileHash .\fixture-48k.f32 -Algorithm SHA256
```

## Render both profiles

Maximum safe suppression (`Vocal = 0`):

```powershell
cargo run --quiet -p voxveil-dsp --example classic_dsp_raw -- --input .\fixture-48k.f32 --output .\fixture-music.f32 --sample-rate 48000 --vocal 0 --profile music-preservation
cargo run --quiet -p voxveil-dsp --example classic_dsp_raw -- --input .\fixture-48k.f32 --output .\fixture-balanced.f32 --sample-rate 48000 --vocal 0 --profile balanced
```

Convert the latency-compensated raw outputs back to WAV for listening or analysis:

```powershell
ffmpeg -v error -f f32le -ac 2 -ar 48000 -i .\fixture-music.f32 -c:a pcm_f32le -y .\fixture-music.wav
ffmpeg -v error -f f32le -ac 2 -ar 48000 -i .\fixture-balanced.f32 -c:a pcm_f32le -y .\fixture-balanced.wav
```

The raw renderer removes the processor's fixed startup latency and flushes its tail, so the output frame count should equal the input frame count. Verify byte counts match before comparing aligned samples:

```powershell
(Get-Item .\fixture-48k.f32).Length
(Get-Item .\fixture-music.f32).Length
(Get-Item .\fixture-balanced.f32).Length
```

## Listening protocol

Level-match before judging quality. Do not treat a quieter result as automatically better vocal reduction.

For each profile record:

- residual lead-vocal audibility;
- loss of centered instruments;
- bass/kick/snare damage;
- transient smearing or pumping;
- stereo-width change;
- metallic/phase artifacts;
- reverb/chorus behavior;
- mono-compatibility change.

Use blind or randomized A/B order when practical.

## Measurement protocol

For fixtures with clean stems, render the exact original mix and keep stems time-aligned. Measure at minimum:

1. **Vocal attenuation** — RMS or LUFS change of vocal-dominant regions between the original mix and processed output, supported by stem-assisted inspection where available.
2. **Accompaniment damage** — difference energy and listening checks in accompaniment-dominant regions, especially centered bass/percussion/instruments.
3. **Stereo preservation** — left/right correlation and side-energy change before/after processing.
4. **Peak safety** — confirm finite output and note any unexpected clipping/overs.
5. **Latency alignment** — raw input/output byte counts match and impulse/transient positions remain aligned after renderer compensation.

Do not report a single separation score as proof of quality. Classic DSP is a bounded stereo-center suppressor, not semantic source separation.

## Runtime measurements

On representative Windows hardware, record:

- CPU model and logical core count;
- Windows version/build;
- source endpoint and physical output;
- sample rate;
- profile and Vocal value;
- process CPU while idle and while processing;
- observed glitch/dropout count;
- end-to-end latency measurement method and result.

Measure both profiles with the same fixture and routing path. Profile switching should not restart the stream.

## Result template

| Fixture | Profile | Vocal | Vocal reduction | Center-instrument damage | Artifacts | Stereo change | CPU | E2E latency | Decision/notes |
| --- | --- | ---: | --- | --- | --- | --- | ---: | ---: | --- |
| fixture-id | Music preservation | 0 |  |  |  |  |  |  |  |
| fixture-id | Balanced | 0 |  |  |  |  |  |  |  |

## Tuning rules

- Music preservation must remain the safer accompaniment-first default.
- Balanced may suppress vocals more strongly but must retain a non-zero center floor.
- Neither profile may hard-delete the stereo center or collapse mono/near-mono material.
- Keep the common 512/128 STFT geometry unless a separately reviewed latency/architecture change is approved.
- Change tuning constants only from repeatable fixture evidence; record the before/after values and affected fixtures in the PR or Wayfinder issue.
