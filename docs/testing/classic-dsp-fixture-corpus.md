# Classic DSP evaluation fixture corpus

This document defines the license/provenance boundary for real-audio evaluation of the **Music preservation** and **Balanced** Classic DSP profiles.

The repository does **not** store downloaded evaluation audio. Keep every downloaded source, derived WAV/raw render, private manifest, and temporary analysis file under `.local-evaluation/classic-dsp/`. The repository ignores `.local-evaluation/`; do not commit any audio from that workspace. Commit only metadata that is intentionally safe to publish, deterministic recipes, aggregate measurements, and source/license references.

A suggested local layout is:

```text
.local-evaluation/classic-dsp/
  sources/
  fixtures/
  renders/
  manifests/
  measurements/
```

Do not place secrets, private download credentials, or license-restricted source audio in tracked documentation as a substitute for the ignored workspace.

## Corpus tiers

Use two separate fixture tiers. They answer different questions and must not be conflated.

### Tier A — controlled quantitative fixtures

Use separately licensed vocal and accompaniment sources to construct deterministic local mixtures with known ground truth.

Approved source families:

| Source | Role | Native rate | License basis | Canonical reference |
| --- | --- | ---: | --- | --- |
| VocalSet | isolated professional singing voice | 44.1 kHz | CC BY 4.0; re-check the exact downloaded record/version before use and retain attribution | https://doi.org/10.5281/zenodo.1203819 |
| URMP | instrumental mixtures + individual instrument tracks | 48 kHz | Dryad research data are released under CC0; retain dataset citation | https://doi.org/10.5061/dryad.ng3r749 |

VocalSet provides male/female clean singing material. URMP provides 44 classical chamber pieces with synchronized individual instrument tracks and assembled mixtures. A controlled fixture may combine one VocalSet excerpt with one URMP accompaniment excerpt after deterministic rate/channel conversion.

Controlled mixtures are useful for:

- exact vocal attenuation measurements;
- accompaniment difference-energy measurements;
- centered-instrument damage checks;
- native 44.1 kHz versus 48 kHz processor comparisons;
- repeatable profile-to-profile regression tests.

They are **not** a substitute for naturally produced music when judging artifacts, reverb behavior, vocal doubles, mastering, or perceived musical quality.

### Tier B — natural-mix subjective fixtures

Use naturally mixed songs only when the exact recording is independently verified as public domain, CC0, CC BY, or covered by another license that permits the intended Voxveil evaluation/reproduction workflow.

For every natural-mix fixture, record the exact per-track license and source. A collection-level marketing statement is insufficient when individual tracks have different rights.

Until a natural-mix track passes that per-file review, mark it `candidate` and do not use it for release acceptance.

## Excluded baseline datasets

Do not use these as Voxveil's redistributable/commercial release baseline unless separate permission for the exact track is documented:

- **MUSDB18 / MUSDB18-HQ** — the official dataset records state educational use only / no commercial use without permission. The collection also includes CC BY-NC-SA material.
- **MedleyDB-derived tracks** — many relevant tracks are CC BY-NC-SA 4.0, which is incompatible with a commercial-safe baseline.
- mirrors or repackaged downloads whose claimed license conflicts with the authoritative upstream dataset/track license.

A permissive license label on a third-party mirror never overrides the original recording's rights.

## Fixture manifest

Keep a local manifest entry under `.local-evaluation/classic-dsp/manifests/` for every evaluated item. A manifest may be copied into tracked evidence only when it contains metadata and hashes, not audio, private paths that expose credentials, or private download credentials.

Required fields:

```json
{
  "fixtureId": "controlled-vocalset-urmp-001",
  "tier": "controlled",
  "status": "approved",
  "targetSampleRate": 44100,
  "durationSeconds": 20.0,
  "sources": [
    {
      "role": "vocal",
      "dataset": "VocalSet",
      "sourceRecord": "https://doi.org/10.5281/zenodo.1203819",
      "sourceFile": "local-relative-or-dataset-file-id.wav",
      "license": "CC-BY-4.0",
      "licenseCheckedOn": "YYYY-MM-DD",
      "sourceSha256": "<sha256>"
    },
    {
      "role": "accompaniment",
      "dataset": "URMP",
      "sourceRecord": "https://doi.org/10.5061/dryad.ng3r749",
      "sourceFile": "AuMix_...wav",
      "license": "CC0-1.0",
      "licenseCheckedOn": "YYYY-MM-DD",
      "sourceSha256": "<sha256>"
    }
  ],
  "mixRecipe": {
    "vocalStartSeconds": 0.0,
    "accompanimentStartSeconds": 0.0,
    "vocalGainDb": -6.0,
    "accompanimentGainDb": 0.0,
    "vocalPan": "center",
    "normalization": "none"
  },
  "fixtureSha256": "<sha256>",
  "notes": "purpose and audible characteristics"
}
```

Do not reuse a result after any source hash, target rate, trim, gain, pan, or mix recipe changes. Treat that as a new fixture/version.

## Deterministic controlled mixing

Use FFmpeg only as an offline fixture-preparation tool. Do not loudness-normalize during fixture construction.

For a 44.1 kHz fixture, preserve VocalSet at 44.1 kHz and deterministically convert the URMP accompaniment to 44.1 kHz. For a 48 kHz fixture, preserve URMP at 48 kHz and deterministically convert the VocalSet excerpt to 48 kHz.

Example 44.1 kHz construction, with a centered mono vocal and explicit gains. Run it from `.local-evaluation/classic-dsp/` or adjust paths while keeping all generated audio inside that ignored workspace:

```powershell
ffmpeg -v error `
  -ss <vocal-start> -t <seconds> -i .\sources\vocal.wav `
  -ss <music-start> -t <seconds> -i .\sources\accompaniment.wav `
  -filter_complex "[0:a]aresample=44100,pan=stereo|c0=c0|c1=c0,volume=-6dB[v];[1:a]aresample=44100,volume=0dB[m];[m][v]amix=inputs=2:duration=shortest:normalize=0[out]" `
  -map "[out]" -ar 44100 -ac 2 -c:a pcm_f32le -map_metadata -1 -y .\fixtures\fixture-44k1.wav
```

Use the equivalent explicit `aresample=48000` / `-ar 48000` recipe for the 48 kHz counterpart. Record the exact command, FFmpeg version, source hashes, and resulting fixture hash in the local manifest.

Do not tune gain values merely to make one profile look better. Select and freeze the mix recipe before comparing profiles.

## Minimum controlled matrix

The quantitative acceptance set should contain, at minimum:

- one male VocalSet source and one female VocalSet source;
- at least one sparse URMP accompaniment;
- at least one dense accompaniment;
- material with strongly centered low-frequency/instrument energy where available;
- at least two independent 44.1 kHz fixtures;
- at least two independent 48 kHz fixtures;
- both Classic DSP profiles at `Vocal = 0`;
- an unprocessed reference for every fixture.

Where practical, include more than one vocal/accompaniment pairing so profile tuning is not driven by one singer or one arrangement.

## Acceptance evidence

For every release-tuning decision, retain:

- fixture manifest/version and SHA-256;
- source/license verification date;
- rendered Music preservation and Balanced hashes;
- vocal attenuation measurement;
- accompaniment damage measurement;
- stereo/peak/alignment checks;
- listening notes;
- CPU/end-to-end latency measurements when run through the real Windows path.

A tuning change is not justified by one fixture or by a single separation score.

## License references

Authoritative/current references used when defining this policy:

- VocalSet dataset record: https://doi.org/10.5281/zenodo.1203819
- Microsoft DNS Challenge source inventory identifying VocalSet as CC BY 4.0: https://github.com/microsoft/DNS-Challenge
- URMP Dryad record: https://doi.org/10.5061/dryad.ng3r749
- Dryad CC0 data policy: https://datadryad.org/help/guides/best_practices
- MUSDB18 official record/license statement: https://doi.org/10.5281/zenodo.1117372

Re-check the exact source and license before acquiring new fixture versions. Dataset hosting and licensing metadata can change independently of Voxveil.
