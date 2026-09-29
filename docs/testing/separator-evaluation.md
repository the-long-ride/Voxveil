# Vocal separator evaluation

## Procedure

1. Acquire stems: see `separator-stem-corpus.md`, then `powershell -File scripts/evaluation/acquire-separator-stems.ps1`.
2. Run `cargo run -p voxveil-dsp --release --example separator_eval -- --pair V,I ... --csv <file> --render <dir>`.
3. Metrics per song (all at Vocal 0, latency compensated):
   - `SDR_inst = 10·log10(Σ i² / Σ (y − i)²)` — output vs clean instrumental, comparable across every profile;
   - `vocal_reduction_dB = 10·log10(Σ v² / Σ v_out²)` — Isolation only, via `SeparatorEvalTap`;
   - `inst_damage_SDR = 10·log10(Σ i² / Σ (i_out − i)²)` — Isolation only;
   - CPU % of one core.
4. CPU bench: `cargo run -p voxveil-dsp --release --example separator_bench`.
5. Tune one parameter family at a time with `--sweep key=v1,v2,...`; pick the value with the best median `SDR_inst` subject to median `inst_damage_SDR ≥ 12 dB`, then confirm by listening. Never change a default from listening alone.

Acceptance (design spec §9.3): `SDR_inst` beats Strong by ≥ 4 dB; `vocal_reduction_dB` ≥ 20; `inst_damage_SDR` ≥ 12; CPU ≤ 5% of one core; listening pass with no musical noise, drum pumping or hollow centered instruments.

## Results

### 2026-09-30 — synthetic only (initial defaults)

No licensed stem corpus has been acquired yet, so these numbers come from synthetic signals and are **not** release evidence.

Unit/integration scene (`crates/voxveil-dsp/tests/vocal_separator.rs`, 48 kHz, 4 s, first second excluded):

| Component | Attenuation at Vocal 0 |
|---|---|
| Centered vibrato voice (220 Hz, 10 harmonics) | 22.1 dB |
| Centered noise-burst drums | 4.4 dB |
| Centered 55 Hz bass | 0.0 dB |
| Pure side 1234 Hz tone | 0.0 dB |

Harness smoke run (synthetic voice over drums + bass + side tone + off-center pad, 6 s, whole file including warm-up):

| Variant | SDR_inst | vocal_reduction | inst_damage | CPU % |
|---|---|---|---|---|
| music-preservation | 7.48 | — | — | 0.52 |
| balanced | 11.04 | — | — | 0.52 |
| strong | 11.72 | — | — | 0.55 |
| isolation | 19.81 | 17.44 | 23.24 | 0.62 |

CPU bench: 60 s of 48 kHz stereo in 0.385 s → 0.64% of one core (release build, development machine).

Open: acquire ≥ 6 licensed pairs, record the baseline table here, run the sweeps, and add a per-song listening verdict. Drum loss on the synthetic scene (4.4 dB) sits close to its 4.5 dB limit; `center_share` 0.2 improved both SDR and damage in the smoke sweep and is the first candidate to confirm on real stems.
