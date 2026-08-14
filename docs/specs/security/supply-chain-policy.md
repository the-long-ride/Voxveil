# Supply-Chain Specification

- No dependency without documented need.
- Commercially compatible permissive license required by default.
- No runtime telemetry dependency.
- No unreviewed lifecycle/install scripts.
- No git dependencies. Registry dependencies must be exact-versioned and policy-approved.
- Rust crates checked by cargo-deny/cargo audit policy.
- JS dependencies installed from frozen lockfile.
- CI actions pinned to immutable SHAs where practical.
- Releases include checksums, SBOM, and license inventory.
- AI model weights require independent exact-checkpoint license approval.
