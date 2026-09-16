# Microsoft Windows Driver Samples provenance

Upstream: https://github.com/microsoft/Windows-driver-samples
Revision: `67d81f217bc01edf7a4320e4911c11065635acfa`
Imported area: `audio/sysvad`
Pinned `audio/sysvad` tree: `6fa502f5bfb3de1395a6c9ffe71e322fd9e28926`
Purpose: source basis for the Voxveil render-only virtual audio driver.

Files derived into `native/windows/driver` retain the applicable upstream copyright and license notices.

The upstream `audio/sysvad` source is materialized at build time and is not vendored in Git. `scripts/windows/import-sysvad-source.ps1` must fetch the exact commit recorded by `SOURCE_REVISION`, verify that `FETCH_HEAD:audio/sysvad` matches `SYSVAD_TREE_SHA`, and only then copy that verified subtree into the ignored `audio/sysvad` working directory.

The materialized directory is disposable build input and may be deleted/recreated at any time. The importer may replace only that generated subtree and must not modify `SOURCE_REVISION`, `SYSVAD_TREE_SHA`, `LICENSE.txt`, or this provenance file.
