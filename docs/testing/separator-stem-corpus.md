# Vocal separator stem corpus

Objective evaluation of the Isolation profile needs separate vocal and instrumental stems. The two stems of a pair do not need to come from the same song: the metrics only need ground truth for what was summed.

Rules (same boundary as `classic-dsp-fixture-corpus.md`):

- Only CC0, CC BY 3.0 or CC BY 4.0 recordings, checked per file at acquisition time. Record attribution.
- No MUSDB18 or other research-only / non-commercial datasets.
- Audio stays in the ignored `.local-evaluation/separator/` workspace. Commit only aggregate numbers.
- Prefer stereo, produced a cappellas (reverb, delay, doubles) — they exercise the side-reverb stage.
- Both stems of a pair must share a sample rate (the harness refuses otherwise).

## Manifest

`.local-evaluation/separator/manifests/stems.json` (local, not committed):

```json
{
  "stems": [
    {
      "id": "example-vocal",
      "role": "vocal",
      "url": "https://ccmixter.org/content/<uploader>/<file>.mp3",
      "sourcePage": "https://ccmixter.org/files/<uploader>/<id>",
      "license": "CC BY 3.0",
      "attribution": "<title> by <artist>",
      "sha256": ""
    }
  ]
}
```

Leave `sha256` empty on first acquisition; `scripts/evaluation/acquire-separator-stems.ps1` pins it, and verifies it on every later run.

## Candidate sources

Start from the `approved-metadata` ccMixter pages in `classic-dsp-fixture-corpus.md`. On each page, confirm the license badge, copy the direct download link of the stem file into the manifest, and record attribution. Add at least three CC BY a cappellas (ccMixter a cappella section) with different singers (male, female, heavily processed) and at least three CC BY instrumentals with centered bass, kick, snare and a centered keys/guitar part.

Target: at least 6 pairs, both 44.1 kHz and 48 kHz represented.

## Running

```powershell
powershell -File scripts/evaluation/acquire-separator-stems.ps1
cargo run -p voxveil-dsp --release --example separator_eval -- --pair V.mp3,I.mp3 --csv .local-evaluation/separator/measurements/run.csv --render .local-evaluation/separator/renders/run
```
