# Guideline

## Repository layout

```text
voxveil/
├── ui/                 React + TypeScript UI
├── tauri/              Tauri application layer; intentionally no tauri/src
├── crates/             shared Rust domain/audio/DSP/routing/model crates
├── locales/            seven bundled locale trees
├── docs/specs/         professional product/architecture/platform specifications
├── scripts/            local quality/build/release tools
└── .github/workflows/  CI, manual build, release
```

## Development

Requirements:

- Node.js 22.16+
- npm 10.9+
- Rust toolchain pinned by `rust-toolchain.toml`
- platform-specific Tauri native prerequisites

Install JavaScript dependencies without executing package lifecycle scripts:

```bash
npm install --ignore-scripts --no-fund
```

Run repository-owned source gates:

```bash
npm run quality
```

Run UI tests and the 85% coverage gate:

```bash
npm run coverage
```

Run the Tauri application:

```bash
npm run dev
```

The current execution environment used to assemble this source had no npm registry access and no Rust toolchain, so `node_modules`, `package-lock.json`, `Cargo.lock`, compiled binaries, and generated mobile projects are intentionally not fabricated. Generate lockfiles on a networked trusted development machine, audit them, then commit them before a production release.
