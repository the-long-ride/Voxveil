# Classic DSP real-audio evaluation

Use this protocol to tune **Music preservation** and **Balanced** without changing their product semantics.

## Fixture requirements

Use `docs/testing/classic-dsp-fixture-corpus.md` as the source/license policy for every real-audio evaluation fixture. Keep downloaded sources, derived fixtures, raw renders, listening WAVs, local manifests, and measurements under `.local-evaluation/classic-dsp/`. That workspace is ignored by Git. Do not commit its audio; only intentionally publishable metadata, deterministic recipes, hashes, aggregate measurements, and source/license references may move into tracked evidence.

Keep the two corpus roles separate:

- **Tier A — controlled quantitative fixtures:** deterministic mixtures with known vocal/accompaniment sources for repeatable attenuation, accompaniment-damage, stereo, peak, alignment, and sample-rate measurements.
- **Tier B — natural-mix subjective fixtures:** naturally produced music whose exact recording license has been reviewed for the intended Voxveil evaluation workflow; use these for artifacts, reverb, doubles, mastering, and musical-quality judgments.

Do not use a candidate natural mix for release acceptance until its exact per-track rights are recorded. Do not use MUSDB18/MUSDB18-HQ or MedleyDB-derived non-commercial material as Voxveil's redistributable/commercial baseline unless separate permission for the exact recording is documented.

Keep each local manifest under `.local-evaluation/classic-dsp/manifests/` and match the corpus policy. At minimum record fixture/version ID, tier/status, source records/files/licenses and license-check dates, source SHA-256 values, target sample rate, deterministic trim/gain/pan/mix recipe, resulting fixture SHA-256, and audible characteristics. Changing a source hash, target rate, trim, gain, pan, or recipe creates a new fixture/version.

The quantitative acceptance set must include at least two independent native 44.1 kHz fixtures and two independent native 48 kHz fixtures, both profiles at `Vocal = 0`, and an unprocessed reference for every fixture. Across the full controlled + natural-mix acceptance set, cover at least:

- sparse vocal + accompaniment;
- dense full-band mix;
- strong centered bass/kick/snare or other centered instruments;
- wide stereo ambience/reverb;
- mono or near-mono material;
- male and female lead vocals;
- harmony/double-tracked vocals where a licensed natural mix permits it;
- native 44.1 kHz and 48 kHz processing paths.

Controlled mixtures support quantitative tuning but do not replace natural production mixes for subjective release acceptance.

## Prepare deterministic input

Run the preparation and render commands from `.local-evaluation/classic-dsp/` unless explicitly noted otherwise. A suggested layout is `sources/`, `fixtures/`, `renders/`, `manifests/`, and `measurements/`.

The primary tuning reference is 48 kHz stereo float32. Convert a prepared fixture without loudness normalization or other processing:

```powershell
ffmpeg -v error -i .\fixtures\fixture.wav -map_metadata -1 -ac 2 -ar 48000 -f f32le -y .\fixtures\fixture-48k.f32
```

Record the SHA-256 of the raw evaluation input so repeated runs use identical bytes:

```powershell
Get-FileHash .\fixtures\fixture-48k.f32 -Algorithm SHA256
```

Do not use only 48 kHz-resampled material for release acceptance. The Windows relay constructs the processor from the endpoint's actual shared sample rate, so the controlled matrix must also include native 44.1 kHz fixtures:

```powershell
ffmpeg -v error -i .\fixtures\fixture-44k1.wav -map_metadata -1 -ac 2 -ar 44100 -f f32le -y .\fixtures\fixture-44k1.f32
Get-FileHash .\fixtures\fixture-44k1.f32 -Algorithm SHA256
```

When constructing a controlled fixture from separately licensed sources, use the frozen mix recipe from `classic-dsp-fixture-corpus.md` rather than ad-hoc gain or normalization changes made after hearing a profile result.

Use `scripts/evaluation/prepare-tier-a-controlled-fixture.ps1` to create Tier-A fixtures and schema-shaped manifests from exact local VocalSet + URMP sources. The helper enforces the native-rate anchor for each target rate and records source/final hashes before any profile render.

## Render both profiles

Maximum safe suppression (`Vocal = 0`) at 48 kHz:

```powershell
cargo run --quiet -p voxveil-dsp --example classic_dsp_raw -- --input .\fixtures\fixture-48k.f32 --output .\renders\fixture-music.f32 --sample-rate 48000 --vocal 0 --profile music-preservation
cargo run --quiet -p voxveil-dsp --example classic_dsp_raw -- --input .\fixtures\fixture-48k.f32 --output .\renders\fixture-balanced.f32 --sample-rate 48000 --vocal 0 --profile balanced
```

For native 44.1 kHz acceptance fixtures, keep the renderer sample-rate argument matched to the raw input rather than resampling them to 48 kHz:

```powershell
cargo run --quiet -p voxveil-dsp --example classic_dsp_raw -- --input .\fixtures\fixture-44k1.f32 --output .\renders\fixture-44k1-music.f32 --sample-rate 44100 --vocal 0 --profile music-preservation
cargo run --quiet -p voxveil-dsp --example classic_dsp_raw -- --input .\fixtures\fixture-44k1.f32 --output .\renders\fixture-44k1-balanced.f32 --sample-rate 44100 --vocal 0 --profile balanced
```

Convert the latency-compensated raw outputs back to WAV for local listening or analysis. These derived audio files remain inside the ignored workspace and are not committed:

```powershell
ffmpeg -v error -f f32le -ac 2 -ar 48000 -i .\renders\fixture-music.f32 -c:a pcm_f32le -y .\renders\fixture-music.wav
ffmpeg -v error -f f32le -ac 2 -ar 48000 -i .\renders\fixture-balanced.f32 -c:a pcm_f32le -y .\renders\fixture-balanced.wav
ffmpeg -v error -f f32le -ac 2 -ar 44100 -i .\renders\fixture-44k1-music.f32 -c:a pcm_f32le -y .\renders\fixture-44k1-music.wav
ffmpeg -v error -f f32le -ac 2 -ar 44100 -i .\renders\fixture-44k1-balanced.f32 -c:a pcm_f32le -y .\renders\fixture-44k1-balanced.wav
```

The raw renderer removes the processor's fixed startup latency and flushes its tail, so the output frame count should equal the input frame count. Verify byte counts match before comparing aligned samples:

```powershell
(Get-Item .\fixtures\fixture-48k.f32).Length
(Get-Item .\renders\fixture-music.f32).Length
(Get-Item .\renders\fixture-balanced.f32).Length
(Get-Item .\fixtures\fixture-44k1.f32).Length
(Get-Item .\renders\fixture-44k1-music.f32).Length
(Get-Item .\renders\fixture-44k1-balanced.f32).Length
```

Record SHA-256 values for the rendered profile outputs in the same evaluation evidence as the input fixture hash.


For a licensed local fixture, the repository helper performs the preparation, both profile renders, latency-compensated byte-count check, finite-sample check, SHA-256 recording, WAV conversion, and basic peak/RMS/mid-side/correlation measurements in one command:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/evaluation/render-classic-dsp-fixture.ps1 `
  -Input .\.local-evaluation\classic-dsp\sources\<source-audio> `
  -FixtureId <fixture-id> `
  -Tier controlled `
  -SampleRate 44100
```

Use `-Tier natural-mix` for an approved natural production mix. The helper requires `ffprobe.exe` and rejects any source whose first audio stream does not already match the requested 44.1/48 kHz rate, so resampled material cannot be mislabeled as native-rate acceptance evidence. Use `-SampleRate 48000` only with an independent native 48 kHz source. The helper writes only under the ignored `.local-evaluation/classic-dsp/{fixtures,renders,measurements}/` workspace. Its JSON evidence records the tier and probed native sample rate and deliberately keeps `subjectiveReview.status = "pending"`; objective rendering/hashes do not constitute listening acceptance, Windows runtime validation, or release qualification.

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

Use blind or randomized A/B order when practical. Controlled fixtures may be included in listening, but release-quality judgments about production artifacts must also include approved Tier B natural mixes.

After listening, record the review through the scoped evidence updater instead of hand-editing the JSON:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/evaluation/record-classic-dsp-listening.ps1 `
  -Evidence .\.local-evaluation\classic-dsp\measurements\<fixture>-44100-render-evidence.json `
  -Decision accepted `
  -MusicPreservation '<level-matched listening notes>' `
  -Balanced '<level-matched listening notes>' `
  -ReviewMethod 'randomized A/B on the same output chain' `
  -Notes '<optional cross-profile notes>'
```

The recorder only edits JSON under the ignored `measurements/` directory, requires notes for both profiles plus an explicit `accepted` or `rejected` decision, timestamps the review, and refuses to overwrite a completed review unless `-Force` is supplied. The decision applies only to that fixture/listening record; it is not a repository-wide release verdict.


## Measurement protocol

Use Tier A controlled fixtures for quantitative comparisons where the source components and mix recipe are known. Measure at minimum:

1. **Vocal attenuation** — RMS or LUFS change of vocal-dominant regions between the original fixture and processed output, supported by the known clean vocal source.
2. **Accompaniment damage** — difference energy and listening checks against the known accompaniment source, especially centered bass/percussion/instruments.
3. **Stereo preservation** — left/right correlation and side-energy change before/after processing.
4. **Peak safety** — confirm finite output and note any unexpected clipping/overs.
5. **Latency alignment** — raw input/output byte counts match and impulse/transient positions remain aligned after renderer compensation.
6. **Sample-rate consistency** — compare independent native 44.1 kHz and 48 kHz acceptance runs for unexpected profile or artifact changes caused only by sample rate.

Use Tier B natural mixes to supplement those measurements with subjective behavior that deterministic stem combinations do not reproduce reliably.

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

Measure both profiles with the same fixture and routing path. Profile switching should not restart the stream. Repeat at both 44.1 kHz and 48 kHz when the target endpoint exposes both shared formats.


Run the local machine-evidence collector from an elevated PowerShell while Voxveil is processing the representative route:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/evaluation/collect-windows-runtime-evidence.ps1 `
  -SampleRate 44100 `
  -Profile music-preservation `
  -Vocal 0 `
  -SourceEndpoint '<source endpoint>' `
  -PhysicalOutput '<physical output>' `
  -WorkloadState processing `
  -CpuSampleSeconds 10
```

The collector writes only under the ignored `.local-evaluation/classic-dsp/measurements/` workspace. It records Windows build/architecture, CPU model and logical-core count, Secure Boot query state, the current BCD `testsigning` value when present, route/profile metadata, the declared `idle`/`processing` workload state, and an optional normalized process-CPU sample. Capture separate CPU samples for idle and processing when both are part of the acceptance record. It intentionally leaves dropout count and end-to-end latency as `null` with `status: "pending"`; those require an actual observed run and an explicit latency measurement method. Run it separately for each required sample-rate/profile route.

After the observed run and latency measurement are complete, finalize the same runtime-evidence JSON with the repository-owned recorder:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/evaluation/record-windows-runtime-measurement.ps1 `
  -Evidence .\.local-evaluation\classic-dsp\measurements\windows-runtime-44100-music-preservation-processing-<timestamp>.json `
  -DropoutCount 0 `
  -EndToEndLatencyMs <measured-ms> `
  -LatencyMethod '<measurement method>' `
  -Notes '<optional run notes>'
```

This updater requires an observed non-negative dropout count, measured latency, and a non-empty method; it timestamps completion and refuses to overwrite a completed measurement unless `-Force` is supplied. Do not use `0` as a placeholder for an unmeasured value.


## Structural evidence audit

After fixture/render/listening and runtime JSON have been recorded, run the repository audit:

```powershell
npm run evaluation:audit -- --workspace .\.local-evaluation\classic-dsp
```

The audit fails closed unless it finds at least two accepted controlled fixtures at 44.1 kHz, two at 48 kHz, at least one accepted non-candidate natural mix, and one **single machine/route group** containing sampled idle + processing CPU evidence for both profiles at both rates. Processing records must also contain completed dropout and end-to-end latency measurements.

This is only a structural gate. It deliberately does **not** infer singer gender, sparse/dense arrangement, centered-instrument content, stereo ambience, mono compatibility, or harmony/double coverage from file names or notes; those semantic corpus checks remain explicit human review items.

## Result template

| Fixture | Tier | Rate | Profile | Vocal | Vocal reduction | Center-instrument damage | Artifacts | Stereo change | CPU | E2E latency | Decision/notes |
| --- | --- | ---: | --- | ---: | --- | --- | --- | --- | ---: | ---: | --- |
| controlled-48k-001 | A | 48000 | Music preservation | 0 |  |  |  |  |  |  |  |
| controlled-48k-001 | A | 48000 | Balanced | 0 |  |  |  |  |  |  |  |
| controlled-44k1-001 | A | 44100 | Music preservation | 0 |  |  |  |  |  |  |  |
| controlled-44k1-001 | A | 44100 | Balanced | 0 |  |  |  |  |  |  |  |
| natural-approved-001 | B | native | Music preservation | 0 | subjective | subjective |  |  |  |  |  |
| natural-approved-001 | B | native | Balanced | 0 | subjective | subjective |  |  |  |  |  |

## Tuning rules

- Music preservation must remain the safer accompaniment-first default.
- Balanced may suppress vocals more strongly but must retain a non-zero center floor.
- Neither profile may hard-delete the stereo center or collapse mono/near-mono material.
- Keep the common 512/128 STFT geometry unless a separately reviewed latency/architecture change is approved.
- Do not tune only against 48 kHz-resampled fixtures; retain independent native 44.1 kHz and 48 kHz acceptance coverage.
- Do not tune from candidate/unreviewed natural mixes or from non-commercial baseline datasets without documented permission.
- Do not tune from one fixture or one singer/arrangement; use the minimum controlled matrix plus approved natural mixes.
- Change tuning constants only from repeatable fixture evidence; record the before/after values, affected fixture IDs/versions, and rendered hashes in the PR or Wayfinder issue.
