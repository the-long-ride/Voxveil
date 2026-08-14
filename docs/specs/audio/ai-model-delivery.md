# AI Model Delivery Specification

## Purpose

Voxveil does not bundle AI model weights in installers or release artifacts. AI model acquisition is an explicit, user-initiated operation that downloads an approved model directly to device-local application data.

## User flow

1. Engine screen shows the approved model, declared license, source, and approximate size.
2. No network request occurs during startup or status checks.
3. User selects **Install AI model**.
4. Voxveil displays a consent dialog explaining the third-party download, local storage, license, and network contact.
5. The download action remains disabled until the user checks the agreement control.
6. Rust receives `acceptedTerms=true`; the backend rejects any request without explicit consent.
7. Voxveil downloads only the catalog-pinned URL, verifies the byte limit and SHA-256, and installs the model only after verification succeeds.
8. The model can be removed at any time from the Engine screen.

## Storage

Models are stored below Tauri's `app_local_data_dir()` under:

```text
<app-local-data>/models/<model-id>/
├── <model-file>.onnx
└── install-receipt.txt
```

A temporary `<model-file>.onnx.download` is used while receiving bytes. Partial or hash-mismatched files are never promoted to the installed path.

## Security boundary

The UI cannot provide a URL. The Rust downloader accepts only model IDs present in `tauri/models/catalog.json`.

Each catalog entry must include:

- exact model ID and filename;
- HTTPS download URL pinned to a 40-character source revision;
- exact SHA-256;
- hard maximum byte count;
- declared commercial-use approval;
- declared license and source;
- `directDownloadOnly=true`.

Redirects are followed manually and only when the next URL is HTTPS. A maximum of five redirects is allowed. No generic Tauri HTTP plugin is enabled.

## Privacy

The model host receives the normal network metadata required for the user-requested HTTPS download. Voxveil does not upload audio, analytics, telemetry, crash data, diagnostics, settings, or model usage during this operation.

## Bundling rule

`.onnx`, `.pt`, `.pth`, `.ckpt`, `.safetensors`, `.tflite`, and `.mlmodel` files are rejected by the repository model-delivery quality gate. Tauri bundle resources are also checked for model binaries.

## Runtime integration boundary

Model acquisition and model inference are separate concerns. This feature establishes trusted acquisition/storage. The ONNX inference implementation must still validate the installed model hash before opening a runtime session.
