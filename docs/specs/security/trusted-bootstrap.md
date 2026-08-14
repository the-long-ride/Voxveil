# Trusted Dependency Bootstrap

This procedure is required the first time lockfiles are created or whenever an approved direct dependency version changes.

## Preconditions

- Use a trusted development machine and a trusted registry/network path.
- Verify Node.js `24.17.0`, npm `10.9.2`, and the Rust toolchain from `rust-toolchain.toml`.
- Review every direct dependency change against both dependency allowlists before resolution.
- Keep npm lifecycle scripts disabled. Do not temporarily bypass `.npmrc` to make installation succeed.

## JavaScript lockfile

```bash
npm install --package-lock-only --ignore-scripts --no-fund
npm ci --ignore-scripts --no-fund --no-audit
npm audit --audit-level=high
npm run quality
npm run typecheck
npm run coverage
npm run build
```

Review `package-lock.json` for unexpected registry hosts, git/tarball dependencies, and surprising native packages before commit.

## Rust lockfile

```bash
cargo generate-lockfile
cargo metadata --locked --format-version=1 > /tmp/voxveil-cargo-metadata.json
cargo deny check
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo llvm-cov --workspace --all-features --fail-under-lines 85
```

Review `Cargo.lock` and Cargo metadata for unexpected sources. Git dependencies are forbidden by the repository-owned static gate.

## Commit rule

`package-lock.json` and `Cargo.lock` must be committed together before CI, manual-build, or release workflows are allowed to proceed.

Never hand-write or fabricate either lockfile.
