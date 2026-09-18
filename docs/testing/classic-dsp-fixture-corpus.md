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

The rights review must cover both the sound recording and any underlying composition/lyrics that are not authored by the recording uploader. A CC-licensed cover recording is not sufficient by itself when the underlying song has separate rights. If composition/lyric rights cannot be established for the intended workflow, keep the track out of release acceptance even when the recording page shows a permissive CC license.

Until a natural-mix track passes that per-file review, mark it `candidate` and do not use it for release acceptance.

#### Approved metadata candidates

These tracks passed the current metadata/source-chain review. `approved-metadata` means they may be acquired into the ignored evaluation workspace; it does **not** mean a local fixture has been created, hashed, rendered, listened to, or accepted for release. Re-check every page/license at acquisition time and record the downloaded file hash before changing the status to an evaluated fixture.

| Track | Native rate | Vocal material | Recording/source-chain basis | Status |
| --- | ---: | --- | --- | --- |
| **Blackout Romeo** — Stefan Kartenberg feat. Thespinwires | 48 kHz | male | remix page is CC BY 3.0 and identifies one source, **Blackout Romeo Vocals** by Thespinwires; that source page is also CC BY 3.0 and is presented as the uploader's vocal line from The Spin Wires song | `approved-metadata` |
| **Road Back To You** — Allerlei von Nicolai feat. Admiral Bob | 44.1 kHz | male | remix page is CC BY 3.0, explicitly says the track may be used for any purpose with attribution, and identifies the vocal source **Love is my Road Back to You** by Admiral Bob; that original-song source is CC BY 3.0 | `approved-metadata` |
| **OUTCAST GROUNDED DREAMS - SKYE JORDAN FT. QUIANA NADINE** — QuianaNadine | 44.1 kHz | female | remix page is CC BY 4.0 and identifies one backing source, **Outcast (Grounded Dreams Edition)** by Skye Jordan; that instrumental source page is also CC BY 4.0 | `approved-metadata` |
| **The Muffin Man (performed by Sinclair Ukiri)** — SinclairUkiri | 44.1 kHz | mixed vocal + piano | Wikimedia Commons identifies the upload as the performer's own work under CC BY 4.0; MediaWiki imageinfo reports stereo Vorbis at 44.1 kHz, 19.6875 s, 1,101,160 bytes, SHA-1 `0a2ff6ab77db7995c4a8b2ed868520bd80838574`; the underlying traditional rhyme was written down by 1820, supporting a public-domain composition basis | `approved-metadata` |

Canonical pages used for the current review:

- https://ccmixter.org/files/JeffSpeed68/55797
- https://ccmixter.org/files/Thespinwires/55741
- https://ccmixter.org/files/Allerlei_von_Nicolai/61840
- https://ccmixter.org/files/admiralbob77/61493
- https://ccmixter.org/files/QuianaNadine/68995
- https://ccmixter.org/files/SkyeJordan/68149
- https://commons.wikimedia.org/wiki/File:The_Muffin_Man_(performed_by_Sinclair_Ukiri).ogg
- https://www.londonmuseum.org.uk/visit/families/rhymes-in-time/the-muffin-man/

For the SinclairUkiri Commons candidate, the API-reported SHA-1 above is discovery metadata only. The original binary has **not** been acquired in the Voxveil evaluation workspace from this runtime, so no local SHA-256, fixture render, listening result, or release-acceptance claim exists yet. Acquire the original file, compute and record its SHA-256, and re-check the source/license before promoting it beyond `approved-metadata` or treating it as evaluated evidence.

A local acquisition helper is provided for this exact candidate:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/evaluation/acquire-tier-b-muffin-man.ps1
```

It downloads only into the ignored `.local-evaluation/classic-dsp/` workspace, verifies the authoritative 1,101,160-byte size and MediaWiki SHA-1 before accepting the file, computes the local SHA-256, and writes a schema-shaped natural-mix manifest that remains `approved-metadata` until listening/evaluation is performed. Use `-Force` to reacquire the original file rather than reusing an already verified local copy.

Do not infer approval from another ccMixter upload merely because its final remix page says CC BY. Trace every listed source and reject unresolved license conflicts. For example, **Waking Me Softly Featuring SnowFlake** currently has conflicting license metadata between its ccMixter page and the ccMixter Bandcamp release, so it is not an approved Voxveil acceptance fixture until that conflict is resolved.

## Excluded baseline datasets

Do not use these as Voxveil's redistributable/commercial release baseline unless separate permission for the exact track is documented:

- **MUSDB18 / MUSDB18-HQ** — the official dataset records state educational use only / no commercial use without permission. The collection also includes CC BY-NC-SA material.
- **MedleyDB-derived tracks** — many relevant tracks are CC BY-NC-SA 4.0, which is incompatible with a commercial-safe baseline.
- CC-licensed cover recordings when the underlying composition/lyrics rights are not independently suitable for the intended workflow. **Sixteen Tons** is not approved from the ccMixter recording license alone because it is explicitly presented as a cover and the composition has separate rights records.
- mirrors or repackaged downloads whose claimed license conflicts with the authoritative upstream dataset/track license.

A permissive license label on a third-party mirror never overrides the original recording's rights.

## Fixture manifest

Keep a local manifest entry under `.local-evaluation/classic-dsp/manifests/` for every evaluated item. A manifest may be copied into tracked evidence only when it contains metadata and hashes, not audio, private paths that expose credentials, or private download credentials.

Every local manifest must conform to [`classic-dsp-fixture-manifest.schema.json`](classic-dsp-fixture-manifest.schema.json). The schema is the machine-readable core shape; this policy remains authoritative for licensing, acquisition, status, and release-acceptance rules. Use `tier: "controlled"` for Tier A fixtures and `tier: "natural-mix"` for Tier B fixtures.

The core required fields are:

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
      "sourceSha256": "<64-hex-sha256>"
    },
    {
      "role": "accompaniment",
      "dataset": "URMP",
      "sourceRecord": "https://doi.org/10.5061/dryad.ng3r749",
      "sourceFile": "AuMix_...wav",
      "license": "CC0-1.0",
      "licenseCheckedOn": "YYYY-MM-DD",
      "sourceSha256": "<64-hex-sha256>"
    }
  ],
  "mixRecipe": {
    "vocalStartSeconds": 0.0,
    "accompanimentStartSeconds": 0.0,
    "vocalGainDb": -6.0,
    "accompanimentGainDb": 0.0,
    "vocalPan": "center",
    "normalization": "none",
    "preparationCommand": "ffmpeg ...",
    "preparationToolVersion": "ffmpeg <version>"
  },
  "referenceFiles": {
    "vocal": {
      "rawFile": "fixtures/controlled-vocalset-urmp-001-vocal-reference.f32",
      "sha256": "<64-hex-sha256>",
      "bytes": 7056000
    },
    "accompaniment": {
      "rawFile": "fixtures/controlled-vocalset-urmp-001-accompaniment-reference.f32",
      "sha256": "<64-hex-sha256>",
      "bytes": 7056000
    }
  },
  "fixtureSha256": "<64-hex-sha256>",
  "notes": "purpose and audible characteristics"
}
```

The angle-bracket hash/date/version values above are documentation placeholders and must be replaced with real values before schema validation. Controlled fixtures also require aligned gained vocal/accompaniment `referenceFiles` so quantitative projection and accompaniment-error metrics can be reproduced from the same frozen mix recipe. For a `natural-mix` fixture, set `mixRecipe` to `null` and omit `referenceFiles` because the fixture is the licensed source mix rather than a locally constructed vocal/accompaniment mixture. Do not use `null` to skip documenting preparation of a controlled fixture.

Do not reuse a result after any source hash, target rate, trim, gain, pan, or mix recipe changes. Treat that as a new fixture/version.

## Deterministic controlled mixing

Use FFmpeg only as an offline fixture-preparation tool. Do not loudness-normalize during fixture construction.

For a 44.1 kHz fixture, preserve VocalSet at 44.1 kHz and deterministically convert the URMP accompaniment to 44.1 kHz. For a 48 kHz fixture, preserve URMP at 48 kHz and deterministically convert the VocalSet excerpt to 48 kHz.

A repository helper freezes that recipe, source hashing, license metadata, and manifest generation. Put the exact licensed VocalSet and URMP files under `.local-evaluation/classic-dsp/sources/`, then run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/evaluation/prepare-tier-a-controlled-fixture.ps1 `
  -VocalSource .\.local-evaluation\classic-dsp\sources\<vocalset-file>.wav `
  -AccompanimentSource .\.local-evaluation\classic-dsp\sources\<urmp-file>.wav `
  -FixtureId controlled-44k1-001 `
  -TargetSampleRate 44100 `
  -DurationSeconds 20 `
  -VocalGainDb -6 `
  -AccompanimentGainDb 0 `
  -VocalSourceRecord '<exact VocalSet record/version>' `
  -AccompanimentSourceRecord '<exact URMP record/version>' `
  -LicenseCheckedOn YYYY-MM-DD
```

For 44.1 kHz the helper requires the VocalSet source itself to be native 44.1 kHz; for 48 kHz it requires the URMP accompaniment itself to be native 48 kHz. VocalSet input must be mono. The other source is deterministically resampled to the target rate. The helper refuses outputs that already exist unless `-Force` is explicit, writes only under the ignored workspace, hashes both sources and the final float32 WAV, emits aligned gained stereo-f32 vocal/accompaniment references, and records `status: prepared` because DSP rendering/listening acceptance still has to happen.

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
- ccMixter natural-mix/source pages listed in the Tier B candidate table above

Re-check the exact source and license before acquiring new fixture versions. Dataset hosting and licensing metadata can change independently of Voxveil.
