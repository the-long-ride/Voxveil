# AI Model Delivery Specification

## Purpose

Windows packages bundle both approved AI models: MDX HQ3 defaults to player use,
while the opt-in system-wide relay always uses Spleeter. Build-time acquisition
uses catalog-pinned URLs and verifies SHA-256 and size limits. The models add about
106 MB; binaries are never committed to git. When bundled files are missing or
invalid, explicit user-initiated download to local application data is the fallback.

## User flow

1. Settings and Engine screens show the approved model, declared license, source, and approximate size. Verified bundled models are marked installed and bundled; Remove is hidden and rejected by the backend.
2. No network request occurs during startup or status checks.
3. User selects **Install AI model**.
4. Voxveil displays a consent dialog explaining the third-party download, local storage, license, and network contact.
5. The download action remains disabled until the user checks the agreement control.
6. Rust receives `acceptedTerms=true`; the backend rejects any request without explicit consent.
7. Voxveil downloads only the catalog-pinned URL, verifies the byte limit and SHA-256, and installs the model only after verification succeeds.
8. Downloaded copies can be removed when no valid bundled copy exists. Bundled resources are never removed by model commands.

## Storage

Models are stored below Tauri's `app_local_data_dir()` under:

```text
<app-local-data>/models/<model-id>/
├── <model-file>.onnx
└── install-receipt.txt
```

A temporary `<model-file>.onnx.download` is used while receiving bytes. Partial or hash-mismatched files are never promoted to the installed path.

Bundled files live at `<resource_dir>/models/<model-id>/<model-file>.onnx` and do
not require a download receipt. Every file must pass catalog size and SHA-256
checks. Bundled files take precedence over app-data copies, which still require
both a receipt and matching hashes. An invalid bundle falls back to a valid
app-data installation or reports not installed; startup never downloads anything.

## Security boundary

The UI cannot provide a URL. The Rust downloader accepts only model IDs present in `tauri/models/catalog.json`.

Each catalog entry must include:

- exact model ID and filename;
- HTTPS download URL pinned to a 40-character source revision;
- exact SHA-256;
- hard maximum byte count;
- declared commercial-use approval;
- declared license and source;
- `directDownloadOnly=true` (fallback downloads contact the catalog source directly);
- `bundledByDefault=true` for the models fetched by Windows packaging.

Redirects are followed manually and only when the next URL is HTTPS. A maximum of five redirects is allowed. No generic Tauri HTTP plugin is enabled.

## Privacy

The model host receives the normal network metadata required for the user-requested HTTPS download. Voxveil does not upload audio, analytics, telemetry, crash data, diagnostics, settings, or model usage during this operation.

## Bundling rule

Committed `.onnx`, `.pt`, `.pth`, `.ckpt`, `.safetensors`, `.tflite`, `.mlmodel`
files are rejected by the repository model-delivery quality gate. Static Tauri
resource lists may not point to unverified model binaries. The Windows staging
script generates `target/ai-model-resources.json`, mapping only the verified
`target/bundled-ai-models/` tree to `models/`. Both package paths supply that Tauri
config and copy the same resources into portable packages before checksumming.
The manual workflow caches hash-addressed downloads, never verification results.

## Runtime integration boundary

Model acquisition and model inference are separate concerns. This feature establishes trusted acquisition/storage. The ONNX inference implementation must still validate the installed model hash before opening a runtime session.
