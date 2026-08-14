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

## AI model download exception

Voxveil does not bundle AI model weights. The only permitted production network path is the narrow model downloader in `tauri/models/download.rs`. It may fetch only a checked-in catalog entry after explicit user consent. Each entry must use HTTPS, pin an immutable source revision, declare a maximum size, and provide an expected SHA-256 digest. The downloaded bytes are staged under application-local data and are promoted only after integrity verification.

The UI cannot provide arbitrary download URLs. Generic HTTP plugins, raw socket APIs, telemetry, analytics, crash uploads, and background model downloads remain prohibited. Model acquisition and AI inference are separate capabilities; downloading a model does not imply that an inference runtime is available.
