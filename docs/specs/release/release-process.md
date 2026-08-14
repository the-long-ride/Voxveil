# Release Process

## CI

Runs on every push to `master` and every pull request targeting `master`.

## Manual Build

`workflow_dispatch` supports selectable ref, platform, and edition. It builds artifacts without creating a GitHub Release.

Platforms: Windows, Linux, macOS, Android, iOS.
Editions: Standard, Pro System.

## Release

A push of a tag matching `v*.*.*` starts the release workflow. The workflow validates strict semantic version syntax, runs full gates, builds available platform/edition artifacts, generates checksums/SBOM/license inventory, signs when configured, and creates the GitHub Release.
