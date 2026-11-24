# voxveil-ai

Buffered CPU stem separation implementing `voxveil_model_api::SeparationEngine`.
The application does not select or wire these engines yet; `AI_RUNTIME_AVAILABLE`
remains false until player/relay integration. Run inference on a worker, not an
audio callback: model loading, chunk inference and resampling allocate/block.

`AiEngine::load(Model::Spleeter, 48000, model_directory, runtime_library)` loads
the two local `*.fp16.onnx` files. `Model::MdxInstHq3` loads `mdx-inst-hq3.onnx`.
Only 44.1/48 kHz interleaved stereo f32 is accepted. Pass an absolute app-owned
runtime path; on Windows use the executable directory's `onnxruntime.dll`, never
an unqualified DLL/PATH search. All instances must use the same runtime library.
Windows x64 package builders stage the hash-pinned CPU runtime and its notices
beside the executable. Models remain consent-based downloads, never bundled.

The engine resamples with rubato's band-limited FFT converter to/from 44.1 kHz,
uses model-specific STFT/masking, and joins fixed chunks with a 16,384-frame
raised-cosine overlap. `process` consumes/produces equally sized buffers with
initial silence. `latency_frames()` is the full host-rate model chunk: 11.958 s
for Spleeter, 5.921 s for MDX. This is buffering latency, not a bound on CPU time
or device/queue latency. `finish()` drains exactly that delay; remove initial
latency when rendering a file. Construct a fresh engine after a discontinuity.
Vocals are the input minus the crossfaded accompaniment, so the stems sum to the
delayed input. On backend failure discard the engine; no silent dry fallback.

## Runtime decision (8 visible Xeon vCPUs, AVX2)

Release `tract-onnx` 0.23.10 real-audio tensor inference took 24.550/24.777 s
for **one** shape-specialized Spleeter network per 11.958 s block (RTF
2.053/2.072, lower bounds; both networks are needed), and 16.090/15.858 s
per MDX 5.921 s block (RTF 2.717/2.678), across two consecutive runs each.
Even these inference-only lower bounds exceed 0.35, so ort 2.0.0-rc.11 with
dynamic ONNX Runtime 1.23.2 CPU is selected. Spleeter's original symbolic graph
loads directly in ort; there is no need to modify downloaded/hashed weights.

Real-audio release tests include resampling, STFT, overlap, inference and tail
drain (but exclude loading). On the 29 s test: Spleeter RTF 0.040, vocal
−25.119 dB / accompaniment −0.255 dB / residual −15.537 dB; MDX RTF 1.103,
−36.719 / −0.168 / −17.958 dB. These are joint waveform-projection coefficients,
not source SDR or listening scores. **MDX did not sustain real time on this VM**;
extra buffering alone cannot fix sustained throughput. Keep it optional.

## Tests

`cargo test -p voxveil-ai` covers packet/chunk continuity, non-identical chunk
crossfade, buffer/error handling, latency, resampling and model delivery.
The delivery tests import the app's pure storage/transfer modules so Linux can
exercise multi-file behavior without building Tauri's Windows controller.

```sh
VOXVEIL_ORT_DYLIB=/absolute/path/to/libonnxruntime.so.1.23.2 \
  cargo test --release -p voxveil-ai --test real_models -- --ignored --nocapture --test-threads=1
```

Real-model tests default to `/home/ubuntu/voxveil-vocal-test`: `mix.wav`,
`vocal3.wav`, `accomp.wav` (48 kHz stereo); weights under `ai/` and
`ai/sherpa-onnx-spleeter-2stems-fp16/`. Override with `VOXVEIL_VOCAL_TEST` and
`VOXVEIL_AI_MODEL_DIR`. The CC BY 3.0 source is spinningmerkaba's "When You Go
Away", https://ccmixter.org/files/jlbrock44/32876 and /32877; excerpt 00:51–01:20,
vocal ×3 with common normalization. Model provenance/hashes are in the catalog;
licenses and UVR credit are in the root `THIRD_PARTY_NOTICES.md`.
