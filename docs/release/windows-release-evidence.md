# Windows release evidence archive

The final Windows release audit is intentionally local and fail-closed. After all required external/runtime observations are complete and `npm run evaluation:audit-release` reports `complete`, create a durable evidence-set manifest from the exact release checkout.

## Export

Run:

```powershell
npm run evaluation:export-release-evidence -- --commit <40-hex-exact-commit> --architecture x64 --release-channel retail
```

The default output is written under the ignored `release-metadata/` directory:

```text
release-metadata/
  windows-release-evidence-<commit>-x64-retail.json
  windows-release-evidence-<commit>-x64-retail.json.sha256
```

The exporter refuses to write a manifest unless the unified release audit is complete for the exact current Git checkout.

## What the manifest binds

The exported JSON records:

- exact Voxveil commit, architecture, and release channel;
- unified audit status for the final Windows package, Classic DSP, Tier 2 virtual driver, and applicable Tier 3 APO/CAPX evidence;
- SHA-256 and byte size for every JSON evidence record under the Classic DSP, Windows-driver, and Windows-APO local evidence workspaces;
- selected lifecycle/qualification record names used by the audits;
- SHA-256 and byte size for the final package `release-manifest.json` and `SHA256SUMS.txt`;
- an `evidenceSetSha256` computed from the timestamp-independent manifest core.

The companion `.sha256` file hashes the exported JSON bytes themselves.

The exporter deliberately records only relative evidence paths. It must not publish local absolute paths, credentials, certificate private keys, Partner Center secrets, device serial numbers, or private signing material.

## Archive boundary

`release-metadata/` is ignored by Git. Generated release evidence is not repository source and must not be committed as proof of an external validation event.

For a real release, archive together in the release-record system:

1. the exported evidence manifest and its `.sha256` sidecar;
2. the exact final Windows package;
3. the exact unsigned virtual-driver submission and `submission-manifest.json`;
4. the returned Microsoft-signed virtual-driver package;
5. the trusted signed APO package when Tier 3 is shipped;
6. the externally retained licensed-audio, performance, hardware-matrix, lifecycle, HLK/WHCP, or other Microsoft-approved evidence referenced by the local JSON records.

The manifest does not copy or certify those external artifacts. It provides a deterministic index tying the repository-accepted evidence set to the exact release candidate.
