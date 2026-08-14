# AI Model License and Distribution Policy

Voxveil does not train models as a product requirement and does not need a model to provide Classic DSP processing.

AI weights are not bundled with Voxveil installers. A model may be offered for explicit in-app download only when the exact catalog artifact has been reviewed and all of the following are recorded:

- implementation/code license permits commercial use;
- checkpoint/weight license permits commercial use;
- direct end-user download/use is permitted;
- modification/conversion/quantization rights cover the intended runtime where relevant;
- no non-commercial, research-only, or education-only restriction exists;
- attribution/notice obligations are practical;
- exact source revision and cryptographic hash are recorded;
- model package contains no executable installer or embedded network behavior.

Architecture license and checkpoint license are separate review items. A permissive implementation repository does not automatically license unrelated weights.

## Distribution modes

### Bundled weights

Default policy: **forbidden**. Release archives and installers must contain no AI weight files unless a future specification explicitly changes this policy and the release gate is updated.

### User-requested download

Permitted only for entries in `tauri/models/catalog.json`. The user must explicitly accept the model-download notice before any network request starts. The URL, source revision, maximum size, commercial-use status, and SHA-256 are fixed by the application build.

If any license or provenance condition becomes ambiguous, remove the catalog entry. Do not silently redirect users to community mirrors or arbitrary checkpoints.
