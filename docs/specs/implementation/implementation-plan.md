# Voxveil Full Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> This plan is intentionally written before implementation. No production code should be created until this document has been reviewed.

**Goal:** Build Voxveil, a local-first cross-platform real-time vocal reduction application for Windows, Linux, macOS, Android, and iOS, with a fully usable non-AI DSP engine, optional commercially-safe local AI separation, all-output/per-app routing, physical/virtual outputs, responsive Editorial Monochrome UI, seven bundled languages, hard quality gates, and secure release automation.

**Architecture:** Voxveil is a Tauri 2 application with a React/TypeScript/Vite UI in `/ui`, a custom-layout Tauri Rust package in `/tauri` with no `src` directory, and reusable Rust crates under `/crates`. All audio/DSP/routing state remains in Rust. React is presentation and command/event consumption only. Native platform adapters implement audio capture/output/routing behind narrow Rust traits. Classic DSP is mandatory and AI is optional.

**Tech Stack:** Tauri 2, Rust stable, React, TypeScript, Vite, pnpm 11, plain CSS, i18next/react-i18next, Vitest, Rust built-in test framework, cargo-llvm-cov, cargo-deny, cargo-audit, GitHub Actions.

---

## Global Constraints

- Product name: **Voxveil**.
- Platforms: **Windows, Linux, macOS, Android, iOS**.
- Editions: **Standard** and **Pro System**.
- Processing is **100% local/offline**.
- No analytics SDK.
- No telemetry SDK.
- No remote fonts.
- No cloud inference.
- No required account.
- No audio upload.
- No general-purpose runtime HTTP capability unless a later approved spec explicitly introduces a narrow, auditable use.
- Classic DSP must remain usable with zero AI model installed.
- Any AI model must pass commercial-use, redistribution, modification, and exact-checkpoint license review before shipping.
- Any model with unclear licensing is rejected.
- Calls/VoIP are bypassed by default.
- All Output and Per-App modes are both required.
- User controls vocal intensity and Latency ↔ Quality.
- Physical and virtual processed outputs are required where the platform allows them.
- Source LOC hard gates:
  - `.ts` ≤ **300 physical lines**
  - `.tsx` ≤ **400 physical lines**
  - `.rs` ≤ **300 physical lines**
  - `.css` ≤ **400 physical lines**
- Code coverage:
  - every first-party testable frontend package/domain ≥ **85%**
  - every first-party testable Rust crate/domain ≥ **85% line coverage**
  - frontend statements/branches/functions/lines ≥ **85%**
- Generated/vendor/native-generated files may be excluded only through a documented allowlist checked into source control.
- No LOC exclusion may be added solely to make CI pass.
- UI theme: **Editorial Monochrome / Almost Monochrome**.
- No default gradients, glow, glassmorphism, or decorative dashboards.
- UI must support Light and Dark themes.
- UI must be responsive across desktop, tablet, and phone.
- UI languages:
  - English (`en`)
  - Vietnamese (`vi`)
  - Simplified Chinese (`zh`)
  - Korean (`ko`)
  - Japanese (`ja`)
  - Spanish (`es`)
  - French (`fr`)
- No user-facing string may be hard-coded directly in feature components.
- Repository uses pnpm, not npm/yarn, for JavaScript package management.
- Commit both `pnpm-lock.yaml` and `Cargo.lock`.
- No git URL JavaScript/Rust dependencies.
- No direct tarball URL dependencies.
- No wildcard dependency versions.
- CI installs must be lockfile-frozen.
- New dependencies require:
  1. functional justification;
  2. exact package/source identity;
  3. commercial-compatible license;
  4. maintenance/security review;
  5. transitive dependency review;
  6. install/build-script review;
  7. telemetry/network behavior review.
- TDD is required for new behavior: failing test → verify fail → minimal implementation → verify pass → refactor.
- Production audio callback paths must not perform filesystem I/O, network I/O, UI IPC, blocking locks, model loading, logging formatting, or unbounded allocation.
- Runtime degradation order: High Quality AI → Balanced AI → Fast AI/Hybrid → Classic DSP → Bypass.
- Audio continuity has priority over separation quality.

---

# 1. Repository and Directory Contract

The canonical structure is:

```text
voxveil/
├── package.json
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── deny.toml
├── .editorconfig
├── .gitignore
├── README.md
│
├── ui/
│   ├── package.json
│   ├── index.html
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── vitest.config.ts
│   ├── app/
│   ├── features/
│   │   ├── home/
│   │   ├── apps/
│   │   ├── routing/
│   │   ├── engine/
│   │   └── settings/
│   ├── components/
│   ├── i18n/
│   ├── theme/
│   ├── lib/
│   └── test/
│
├── tauri/
│   ├── package.json
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── tauri.windows.conf.json
│   ├── tauri.linux.conf.json
│   ├── tauri.macos.conf.json
│   ├── tauri.android.conf.json
│   ├── tauri.ios.conf.json
│   ├── main.rs
│   ├── lib.rs
│   ├── app/
│   ├── audio/
│   ├── dsp/
│   ├── separation/
│   ├── routing/
│   ├── platform/
│   │   ├── windows/
│   │   ├── linux/
│   │   ├── macos/
│   │   ├── android/
│   │   └── ios/
│   ├── realtime/
│   ├── config/
│   ├── security/
│   ├── capabilities/
│   ├── icons/
│   └── tests/
│
├── crates/
│   ├── voxveil-types/
│   ├── voxveil-audio-core/
│   ├── voxveil-dsp/
│   ├── voxveil-routing/
│   └── voxveil-model-api/
│
├── locales/
│   ├── en/
│   │   └── common.json
│   ├── vi/
│   │   └── common.json
│   ├── zh/
│   │   └── common.json
│   ├── ko/
│   │   └── common.json
│   ├── ja/
│   │   └── common.json
│   ├── es/
│   │   └── common.json
│   └── fr/
│       └── common.json
│
├── docs/
│   ├── design.md
│   ├── security.md
│   └── specs/
│       ├── product/
│       ├── architecture/
│       ├── audio/
│       ├── platform/
│       ├── security/
│       ├── testing/
│       ├── release/
│       └── implementation/
│
├── scripts/
│   ├── quality/
│   │   ├── check-loc.mjs
│   │   ├── check-loc.test.mjs
│   │   ├── check-i18n.mjs
│   │   ├── check-i18n.test.mjs
│   │   ├── check-network-surface.mjs
│   │   ├── check-network-surface.test.mjs
│   │   └── check-license-metadata.mjs
│   └── release/
│       ├── verify-artifacts.mjs
│       └── write-checksums.mjs
│
└── .github/
    ├── workflows/
    │   ├── ci.yml
    │   ├── manual-build.yml
    │   └── release.yml
    └── pull_request_template.md
```

### Custom Tauri layout rule

`/tauri` intentionally contains `main.rs` and `lib.rs` directly, with no `src/`.

`tauri/Cargo.toml` must explicitly configure:

```toml
[lib]
name = "voxveil_app"
path = "lib.rs"

[[bin]]
name = "voxveil"
path = "main.rs"
```

This is valid Cargo behavior because Cargo supports explicit target paths.

`tauri/package.json` exists only so the Tauri JS CLI can run with `/tauri` as its working directory without relying on the conventional `/src-tauri` directory.

---

# 2. Dependency Policy and Initial Allowlist

## 2.1 JavaScript runtime dependencies

Initial runtime dependency ceiling:

```text
react
react-dom
i18next
react-i18next
@tauri-apps/api
```

No state framework initially. Use React state/context plus small first-party stores.

No router package initially. The app has a small fixed navigation model and can use first-party route state.

No icon package initially. Keep a small set of audited local SVG React components.

No CSS framework.

No Tailwind.

No component framework.

No analytics.

No crash reporting.

No HTTP plugin.

## 2.2 JavaScript development dependencies

Candidates:

```text
typescript
vite
@vitejs/plugin-react
vitest
@vitest/coverage-v8
jsdom
@types/react
@types/react-dom
@tauri-apps/cli
```

Each exact version is accepted only after the lockfile and license report pass the repository gates.

## 2.3 Rust dependency ceiling for foundation

Foundation crates should start with as little as possible:

```text
serde
serde_json
thiserror
tauri
```

Later DSP/runtime candidates:

```text
rustfft
ringbuf
tracing
```

Do not add them before the task that needs them.

## 2.4 Dependency source restrictions

`pnpm-workspace.yaml` must enable supply-chain protections supported by the selected pnpm 11 release:

```yaml
blockExoticSubdeps: true
minimumReleaseAge: 10080
trustPolicy: no-downgrade
```

Use a **7-day** release-age delay for new package resolution. Emergency exceptions require a reviewed `minimumReleaseAgeExclude` entry with a code comment in `docs/security.md`.

Dependency build scripts remain blocked by default. If an approved dependency needs a build script, add only that exact package to pnpm's build allowlist and document why.

## 2.5 Package manager bootstrap

During implementation:

1. Use the machine's existing Corepack only to resolve an exact pnpm 11 version if possible:
   ```bash
   corepack use pnpm@latest-11
   ```
2. Verify:
   ```bash
   pnpm --version
   ```
3. Confirm `package.json` now contains an exact `packageManager` entry.
4. Commit that exact entry.
5. Never keep `latest`, `latest-11`, `^`, or `~` in the committed `packageManager` field.
6. CI uses the exact committed package-manager version.

If Corepack cannot resolve pnpm on a machine, use the official pnpm GitHub Action in CI and a documented local install method. Do not introduce `curl | sh` into repository scripts.

---

# 3. Specification Documents to Create Before Feature Code

The following documents are required and are implementation inputs:

```text
docs/design.md
docs/security.md
docs/specs/product/product-requirements.md
docs/specs/architecture/system-architecture.md
docs/specs/audio/realtime-pipeline.md
docs/specs/audio/classic-dsp.md
docs/specs/audio/model-interface.md
docs/specs/platform/windows.md
docs/specs/platform/linux.md
docs/specs/platform/macos.md
docs/specs/platform/android.md
docs/specs/platform/ios.md
docs/specs/security/supply-chain-policy.md
docs/specs/testing/test-strategy.md
docs/specs/release/build-matrix.md
docs/specs/release/release-process.md
docs/specs/implementation/implementation-plan.md
```

`docs/specs/implementation/implementation-plan.md` will contain this plan when implementation begins.

---

# 4. Theme Contract for docs/design.md

## 4.1 Direction

Name: **Editorial Monochrome**

Character:

- x.ai-like restraint;
- fashion-editorial spacing;
- strong type hierarchy;
- almost monochrome;
- technical information shown quietly;
- visual identity from typography, spacing, rhythm, and motion instead of decoration.

## 4.2 Light tokens

Initial CSS token contract:

```css
:root {
  --vv-bg: #f7f7f5;
  --vv-surface: #ffffff;
  --vv-surface-subtle: #f0f0ed;
  --vv-text: #111111;
  --vv-text-muted: #6c6c68;
  --vv-border: #deded9;
  --vv-control: #171717;
  --vv-control-text: #ffffff;
  --vv-focus: #4f4f55;
  --vv-success: #287a4b;
  --vv-warning: #9a6818;
  --vv-danger: #a13a3a;
}
```

## 4.3 Dark tokens

```css
[data-theme="dark"] {
  --vv-bg: #0d0d0d;
  --vv-surface: #141414;
  --vv-surface-subtle: #1b1b1b;
  --vv-text: #f2f2ef;
  --vv-text-muted: #9b9b96;
  --vv-border: #2b2b29;
  --vv-control: #f1f1ed;
  --vv-control-text: #111111;
  --vv-focus: #b6b6b0;
  --vv-success: #63a87b;
  --vv-warning: #c6984f;
  --vv-danger: #d47474;
}
```

Final contrast values must be validated before merge. Semantic colors are reserved for actual semantic states.

## 4.4 Typography

Use system fonts only:

```css
--vv-font-sans:
  Inter,
  ui-sans-serif,
  -apple-system,
  BlinkMacSystemFont,
  "Segoe UI",
  sans-serif;

--vv-font-mono:
  "SFMono-Regular",
  Consolas,
  "Liberation Mono",
  monospace;
```

Do not bundle Inter unless a later license/asset decision explicitly approves it. If Inter is unavailable, the system stack remains valid.

## 4.5 Spacing

Use a restrained scale:

```text
4, 8, 12, 16, 20, 24, 32, 40, 48, 64
```

## 4.6 Radius

```text
small: 6px
control: 8px
panel: 10px
modal: 12px
```

Avoid pill-heavy UI.

## 4.7 Motion

```text
fast: 120ms
normal: 160ms
slow: 220ms
```

Use opacity/transform only where possible. Respect `prefers-reduced-motion`.

## 4.8 Desktop navigation

```text
Home
Apps
Routing
Engine
Settings
```

## 4.9 Mobile navigation

```text
Home
Apps
Routing
Settings
```

Engine controls are part of Home on mobile.

---

# 5. User-Facing Processing Model

## 5.1 Engines

```text
Auto
Classic DSP
AI
```

Hybrid exists internally and can be exposed later only if UX testing shows value.

## 5.2 Vocal control semantics

For AI separation:

```text
output = accompaniment + vocals × vocalGain
```

For Classic DSP:

```text
slider = vocal suppression strength
```

The UI may use one label, `Vocals`, but help text must accurately explain that DSP estimates and suppresses vocal-like centered content rather than creating a perfect isolated stem.

## 5.3 Quality control

Primary control:

```text
Low Latency  ←────────●────────→  High Quality
```

Internal normalized value:

```rust
pub struct QualityPreference(pub f32); // clamped 0.0..=1.0
```

Preset mapping:

```text
0.00–0.20 Ultra Low Latency
0.20–0.45 Low Latency
0.45–0.70 Balanced
0.70–1.00 High Quality
```

Auto mode may move within the user-selected ceiling but must not silently exceed the user's latency/quality preference.

---

# 6. Core Rust Interfaces

These signatures are the architectural contract for later tasks.

## 6.1 Shared types

`crates/voxveil-types/src/audio.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRate(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelCount(pub u16);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VocalLevel(pub f32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityPreference(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingEngineKind {
    Auto,
    ClassicDsp,
    Ai,
}
```

`VocalLevel` invariant: `0.0..=1.0`.

`QualityPreference` invariant: `0.0..=1.0`.

## 6.2 Audio block

`crates/voxveil-audio-core/src/block.rs`

```rust
pub struct AudioBlock<'a> {
    pub interleaved: &'a mut [f32],
    pub channels: ChannelCount,
    pub sample_rate: SampleRate,
}
```

V1 internal sample representation: interleaved `f32`, nominal range `[-1.0, 1.0]`.

## 6.3 Processor trait

`crates/voxveil-audio-core/src/processor.rs`

```rust
pub trait AudioProcessor: Send {
    fn process(&mut self, block: &mut AudioBlock<'_>) -> ProcessReport;
    fn latency_frames(&self) -> usize;
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessReport {
    pub underrun_risk: bool,
    pub degraded: bool,
}
```

## 6.4 Separation trait

`crates/voxveil-model-api/src/lib.rs`

```rust
pub trait SeparationEngine: Send {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<SeparationReport, SeparationError>;

    fn latency_frames(&self) -> usize;
    fn capabilities(&self) -> SeparationCapabilities;
}
```

## 6.5 Routing trait

`crates/voxveil-routing/src/backend.rs`

```rust
pub trait AudioRoutingBackend: Send {
    fn list_sources(&self) -> Result<Vec<AudioSource>, RoutingError>;
    fn list_outputs(&self) -> Result<Vec<AudioOutput>, RoutingError>;
    fn apply(&mut self, plan: &RoutingPlan) -> Result<(), RoutingError>;
    fn restore(&mut self) -> Result<(), RoutingError>;
}
```

Platform code implements this trait.

---

# 7. Implementation Sequence

The sequence is intentional. Do not start platform driver work before the shared contracts, tests, and baseline DSP are stable.

---

## Task 1: Create Approved Specification Set

**Files:**
- Create: `docs/design.md`
- Create: `docs/security.md`
- Create: all specification files listed in section 3
- Create: `docs/specs/implementation/implementation-plan.md`

**Produces:** The written contract used by all later tasks.

- [ ] **Step 1: Copy the approved product behavior into `product-requirements.md`.**
- [ ] **Step 2: Write `docs/design.md` from section 4 with Light/Dark tokens, responsive rules, component states, accessibility rules, and no decorative additions.**
- [ ] **Step 3: Write `docs/security.md` with local-first, dependency, telemetry, model-license, update, signing, and threat-boundary policies.**
- [ ] **Step 4: Write platform specs that distinguish Standard vs Pro System capabilities and explicitly flag privileged installation requirements.**
- [ ] **Step 5: Copy this plan into `docs/specs/implementation/implementation-plan.md`.**
- [ ] **Step 6: Run a documentation self-review for `TBD`, `TODO`, contradictions, ambiguous requirements, and unsupported product claims.**
- [ ] **Step 7: Commit.**

```bash
git add docs
git commit -m "docs: define Voxveil product and architecture specs"
```

**Acceptance:**
- no `TBD`/`TODO`;
- app name is Voxveil;
- no spec claims perfect DSP isolation;
- mobile privileged constraints are stated;
- all seven languages are listed;
- quality gates match exact requested LOC/coverage limits.

---

## Task 2: Bootstrap pnpm Workspace and Rust Workspace

**Files:**
- Create: `package.json`
- Create: `pnpm-workspace.yaml`
- Create: `ui/package.json`
- Create: `tauri/package.json`
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.editorconfig`
- Create: `.gitignore`

**Produces:** Reproducible workspace skeleton without feature code.

- [ ] **Step 1: Resolve and pin pnpm 11 exactly.**

```bash
corepack use pnpm@latest-11
pnpm --version
```

Verify root `package.json` contains an exact `packageManager`.

- [ ] **Step 2: Configure workspace packages.**

`pnpm-workspace.yaml`:

```yaml
packages:
  - ui
  - tauri

blockExoticSubdeps: true
minimumReleaseAge: 10080
trustPolicy: no-downgrade
```

- [ ] **Step 3: Add root scripts only for orchestration.**

Expected root scripts:

```json
{
  "scripts": {
    "dev": "pnpm --filter @voxveil/tauri-shell tauri dev",
    "build": "pnpm --filter @voxveil/tauri-shell tauri build",
    "test:ui": "pnpm --filter @voxveil/ui test",
    "coverage:ui": "pnpm --filter @voxveil/ui coverage",
    "quality:loc": "node scripts/quality/check-loc.mjs",
    "quality:i18n": "node scripts/quality/check-i18n.mjs",
    "quality:network": "node scripts/quality/check-network-surface.mjs"
  }
}
```

- [ ] **Step 4: Create Rust workspace manifest.**

```toml
[workspace]
resolver = "2"
members = [
  "tauri",
  "crates/voxveil-types",
  "crates/voxveil-audio-core",
  "crates/voxveil-dsp",
  "crates/voxveil-routing",
  "crates/voxveil-model-api",
]
```

- [ ] **Step 5: Pin Rust stable via `rust-toolchain.toml` with `rustfmt` and `clippy` components.**
- [ ] **Step 6: Verify Node/pnpm workspace parsing.**

```bash
pnpm install --lockfile-only
```

Expected: lockfile created; no dependency build scripts executed unexpectedly.

- [ ] **Step 7: Commit.**

```bash
git add package.json pnpm-workspace.yaml pnpm-lock.yaml Cargo.toml rust-toolchain.toml ui/package.json tauri/package.json
git commit -m "build: bootstrap Voxveil workspaces"
```

---

## Task 3: Implement LOC Gate Before Feature Code

**Files:**
- Create: `scripts/quality/check-loc.test.mjs`
- Create: `scripts/quality/check-loc.mjs`

**Interfaces:**

```js
export function limitFor(path) -> number | null
export function countPhysicalLines(text) -> number
export function scan(root) -> Violation[]
```

- [ ] **Step 1: Write failing Node built-in tests.**

Cases:
- `.ts` at 300 passes, 301 fails;
- `.tsx` at 400 passes, 401 fails;
- `.rs` at 300 passes, 301 fails;
- `.css` at 400 passes, 401 fails;
- unsupported extensions ignored;
- `node_modules`, `target`, `dist`, `.git`, generated allowlist paths ignored;
- blank lines still count.

- [ ] **Step 2: Verify RED.**

```bash
node --test scripts/quality/check-loc.test.mjs
```

Expected: FAIL because implementation does not exist.

- [ ] **Step 3: Implement the smallest scanner using only Node built-ins.**
- [ ] **Step 4: Verify GREEN.**

```bash
node --test scripts/quality/check-loc.test.mjs
node scripts/quality/check-loc.mjs .
```

- [ ] **Step 5: Commit.**

```bash
git add scripts/quality
git commit -m "test: enforce source file LOC limits"
```

---

## Task 4: Bootstrap React/Vite UI with Coverage Gate

**Files:**
- Create: `ui/index.html`
- Create: `ui/tsconfig.json`
- Create: `ui/vite.config.ts`
- Create: `ui/vitest.config.ts`
- Create: `ui/app/main.tsx`
- Create: `ui/app/App.tsx`
- Create: `ui/app/App.test.tsx`
- Create: `ui/test/setup.ts`

**Produces:** Minimal testable React shell.

- [ ] **Step 1: Add only the approved UI dependencies and regenerate the lockfile.**
- [ ] **Step 2: Run license/supply-chain inspection before executing tests.**
- [ ] **Step 3: Write a failing test that renders `App` and finds the Voxveil application landmark.**
- [ ] **Step 4: Verify RED.**

```bash
pnpm --filter @voxveil/ui test -- --run
```

- [ ] **Step 5: Implement minimal `main.tsx` and `App.tsx`.**
- [ ] **Step 6: Configure Vitest V8 coverage thresholds:**

```ts
thresholds: {
  lines: 85,
  functions: 85,
  branches: 85,
  statements: 85,
}
```

- [ ] **Step 7: Verify GREEN and coverage.**

```bash
pnpm --filter @voxveil/ui test -- --run
pnpm --filter @voxveil/ui coverage
```

- [ ] **Step 8: Run LOC gate.**
- [ ] **Step 9: Commit.**

---

## Task 5: Implement Editorial Monochrome Theme System

**Files:**
- Create: `ui/theme/tokens.css`
- Create: `ui/theme/base.css`
- Create: `ui/theme/theme.ts`
- Create: `ui/theme/theme.test.ts`
- Create: `ui/components/ThemeProvider.tsx`
- Create: `ui/components/ThemeProvider.test.tsx`

**Interfaces:**

```ts
export type ThemeMode = 'system' | 'light' | 'dark';
export function resolveTheme(mode: ThemeMode, systemDark: boolean): 'light' | 'dark';
```

- [ ] Write failing tests for theme resolution.
- [ ] Verify RED.
- [ ] Implement theme resolution.
- [ ] Verify GREEN.
- [ ] Add approved light/dark CSS tokens.
- [ ] Add tests that theme provider sets `data-theme`.
- [ ] Respect `prefers-color-scheme` and `prefers-reduced-motion`.
- [ ] Verify all CSS files ≤ 400 lines.
- [ ] Commit.

**Visual acceptance:**
- no gradient;
- no glow;
- no glass;
- no giant shadow;
- no accent used decoratively;
- focus ring visible;
- semantic colors only semantic.

---

## Task 6: Implement Responsive App Shell and Navigation

**Files:**
- Create: `ui/app/navigation.ts`
- Create: `ui/app/navigation.test.ts`
- Create: `ui/app/AppShell.tsx`
- Create: `ui/app/AppShell.test.tsx`
- Create: `ui/app/app-shell.css`
- Create: `ui/components/SideNavigation.tsx`
- Create: `ui/components/BottomNavigation.tsx`
- Create: `ui/components/MasterToggle.tsx`

**Navigation:**
Desktop:
- Home
- Apps
- Routing
- Engine
- Settings

Mobile:
- Home
- Apps
- Routing
- Settings

- [ ] Write navigation model tests.
- [ ] Verify RED.
- [ ] Implement pure navigation model.
- [ ] Verify GREEN.
- [ ] Write rendering tests for desktop and mobile nav variants.
- [ ] Implement shell with CSS media queries.
- [ ] Ensure minimum touch target 44×44 CSS pixels.
- [ ] Verify keyboard focus order.
- [ ] Verify no horizontal overflow at 320 CSS px width.
- [ ] Verify LOC and coverage.
- [ ] Commit.

---

## Task 7: Implement i18n Infrastructure and Seven Locales

**Files:**
- Create: `ui/i18n/index.ts`
- Create: `ui/i18n/languages.ts`
- Create: `ui/i18n/i18n.test.ts`
- Create: seven `locales/*/common.json`
- Create: `scripts/quality/check-i18n.test.mjs`
- Create: `scripts/quality/check-i18n.mjs`

**Canonical locale:** English.

**Interfaces:**

```ts
export const SUPPORTED_LANGUAGES = ['en', 'vi', 'zh', 'ko', 'ja', 'es', 'fr'] as const;
export type SupportedLanguage = typeof SUPPORTED_LANGUAGES[number];
```

- [ ] Write failing test that every supported language loads `app.name`, `nav.home`, `processing.vocals`, `settings.language`.
- [ ] Verify RED.
- [ ] Implement local bundled i18n initialization.
- [ ] Verify GREEN.
- [ ] Write quality-script tests detecting missing/extra keys.
- [ ] Implement exact-key parity checker with Node built-ins.
- [ ] Add first complete translation set.
- [ ] Add layout tests using long French/Spanish/Vietnamese strings.
- [ ] Ensure no runtime network loading.
- [ ] Commit.

---

## Task 8: Bootstrap Custom `/tauri` Tauri 2 Shell

**Files:**
- Create: `tauri/Cargo.toml`
- Create: `tauri/build.rs`
- Create: `tauri/main.rs`
- Create: `tauri/lib.rs`
- Create: `tauri/tauri.conf.json`
- Create: `tauri/capabilities/default.json`
- Create: `tauri/app/mod.rs`
- Create: `tauri/app/commands.rs`
- Create: `tauri/app/commands_test.rs` or inline unit tests

**Important:** no `/tauri/src`.

- [ ] Write Rust tests for pure command DTO mapping before command registration.
- [ ] Verify RED with:

```bash
cargo test -p voxveil-tauri
```

- [ ] Configure explicit Cargo `lib` and `bin` paths.
- [ ] Implement minimal `run()` in `lib.rs`.
- [ ] Keep `main.rs` tiny: call `voxveil_app::run()`.
- [ ] Configure Tauri `beforeDevCommand` and `beforeBuildCommand` with working directory `../ui`.
- [ ] Configure `frontendDist` to `../ui/dist`.
- [ ] Use minimal capability allowlist.
- [ ] Do not install/register HTTP, shell, process, or filesystem plugins.
- [ ] Verify `pnpm dev` reaches UI once local OS build prerequisites exist.
- [ ] Verify LOC.
- [ ] Commit.

---

## Task 9: Add Security/License/Network Quality Gates

**Files:**
- Create: `deny.toml`
- Create: `scripts/quality/check-network-surface.test.mjs`
- Create: `scripts/quality/check-network-surface.mjs`
- Create: `scripts/quality/check-license-metadata.mjs`
- Update: `docs/specs/security/supply-chain-policy.md`

**License allowlist baseline:**

```text
MIT
Apache-2.0
MIT OR Apache-2.0
BSD-2-Clause
BSD-3-Clause
ISC
Zlib
Unicode-3.0
```

Any other license needs explicit legal review before allowlisting.

- [ ] Write failing test that catches `@tauri-apps/plugin-http`.
- [ ] Write failing test that catches `fetch(` in production UI outside an explicit approved file list.
- [ ] Write failing test that catches JavaScript dependencies with `git+`, `http:`, `https:` specifiers.
- [ ] Implement scanner.
- [ ] Add `cargo deny check licenses advisories bans sources`.
- [ ] Add `cargo audit`.
- [ ] Add `pnpm audit` as a CI advisory signal/gate for high/critical production vulnerabilities.
- [ ] Generate and review third-party license metadata.
- [ ] Commit.

---

## Task 10: Implement Shared Rust Domain Types

**Files:**
- Create: `crates/voxveil-types/Cargo.toml`
- Create: `crates/voxveil-types/src/lib.rs`
- Create: `crates/voxveil-types/src/audio.rs`
- Create: `crates/voxveil-types/src/processing.rs`
- Create: `crates/voxveil-types/src/routing.rs`

- [ ] Write failing tests for clamping/validation:
  - `VocalLevel::new(-0.1)` returns error;
  - `VocalLevel::new(0.0/1.0)` succeeds;
  - `QualityPreference` same invariant.
- [ ] Verify RED.
- [ ] Implement value objects.
- [ ] Verify GREEN.
- [ ] Add serde only where IPC/config requires serialization.
- [ ] Deny unsafe code in this crate.
- [ ] Verify LOC.
- [ ] Commit.

---

## Task 11: Implement Audio Core and RT-Safe Buffer Boundary

**Files:**
- Create: `crates/voxveil-audio-core/Cargo.toml`
- Create: `crates/voxveil-audio-core/src/lib.rs`
- Create: `crates/voxveil-audio-core/src/block.rs`
- Create: `crates/voxveil-audio-core/src/processor.rs`
- Create: `crates/voxveil-audio-core/src/buffer.rs`
- Create: `crates/voxveil-audio-core/src/buffer_test.rs`

**Behavior:**
- fixed-capacity queues;
- no allocation during steady-state push/pop;
- overflow is explicit;
- underflow is explicit;
- no panic for normal audio pressure.

- [ ] Write failing tests for FIFO order.
- [ ] Write failing overflow/underflow tests.
- [ ] Verify RED.
- [ ] Audit `ringbuf` exact version/license/source before adding it; if rejected, implement a small internal SPSC wrapper around an audited alternative.
- [ ] Implement buffer abstraction.
- [ ] Verify GREEN.
- [ ] Benchmark 48 kHz stereo block movement.
- [ ] Document measured behavior.
- [ ] Commit.

---

## Task 12: Implement Classic DSP v1 — Mid/Side Suppression

**Files:**
- Create: `crates/voxveil-dsp/Cargo.toml`
- Create: `crates/voxveil-dsp/src/lib.rs`
- Create: `crates/voxveil-dsp/src/mid_side.rs`
- Create: `crates/voxveil-dsp/src/mid_side_test.rs`
- Create: `crates/voxveil-dsp/src/fixtures.rs`

**Mathematical baseline:**

```text
mid  = (L + R) * 0.5
side = (L - R) * 0.5

reduced_mid = mid * (1 - suppression)

L' = reduced_mid + side
R' = reduced_mid - side
```

- [ ] Write test: suppression 0.0 is bit-close to input.
- [ ] Write test: fully centered mono content is strongly attenuated at 1.0.
- [ ] Write test: pure side content is preserved.
- [ ] Write test: no NaN/Inf for valid finite input.
- [ ] Verify RED.
- [ ] Implement.
- [ ] Verify GREEN.
- [ ] Add golden WAV-fixture test data only if fixture licensing is owned/compatible; otherwise generate deterministic synthetic signals in tests.
- [ ] Commit.

---

## Task 13: Implement Classic DSP v2 — Frequency-Selective STFT Mask

**Files:**
- Create: `crates/voxveil-dsp/src/stft.rs`
- Create: `crates/voxveil-dsp/src/vocal_mask.rs`
- Create: `crates/voxveil-dsp/src/stft_test.rs`
- Update: `docs/specs/audio/classic-dsp.md`

**Dependency candidate:** `rustfft`, only after exact version passes license/source/advisory gates.

**Algorithm goal:**
- preserve low-frequency centered bass better than simple center cancel;
- reduce center-correlated energy mostly in vocal-relevant bands;
- overlap-add reconstruction;
- suppression amount controlled continuously.

- [ ] Write reconstruction test: suppression 0.0 returns signal within documented error tolerance.
- [ ] Write DC/impulse/sine stability tests.
- [ ] Write stereo-side preservation test.
- [ ] Write block-boundary continuity test.
- [ ] Verify RED.
- [ ] Implement Hann window + overlap-add.
- [ ] Implement conservative center-correlation mask.
- [ ] Verify GREEN.
- [ ] Benchmark latency for selected frame sizes.
- [ ] Record latency table in audio spec.
- [ ] Commit.

---

## Task 14: Implement Engine Coordinator and Quality Preference

**Files:**
- Create: `tauri/audio/engine.rs`
- Create: `tauri/audio/quality.rs`
- Create: `tauri/audio/engine_test.rs`
- Create: `tauri/realtime/health.rs`

**Interfaces:**

```rust
pub struct EngineSettings {
    pub kind: ProcessingEngineKind,
    pub vocal_level: VocalLevel,
    pub quality: QualityPreference,
}
```

- [ ] Write tests for quality tier mapping.
- [ ] Write tests for fallback sequence.
- [ ] Write tests that user-selected Classic DSP never silently switches to AI.
- [ ] Write tests that Auto can degrade but does not exceed user quality ceiling.
- [ ] Verify RED.
- [ ] Implement coordinator using Classic DSP only initially.
- [ ] Verify GREEN.
- [ ] Commit.

---

## Task 15: Implement Routing Domain and Global/Per-App Override Resolution

**Files:**
- Create: `crates/voxveil-routing/Cargo.toml`
- Create: `crates/voxveil-routing/src/lib.rs`
- Create: `crates/voxveil-routing/src/model.rs`
- Create: `crates/voxveil-routing/src/resolve.rs`
- Create: `crates/voxveil-routing/src/resolve_test.rs`
- Create: `crates/voxveil-routing/src/backend.rs`

**Resolution order:**

```text
Global Defaults
→ Mode Defaults
→ Per-App Overrides
```

**Required rule:** calls/VoIP bypass by default.

- [ ] Write tests for global all-output mode.
- [ ] Write tests for per-app on/off.
- [ ] Write tests for partial per-app override inheritance.
- [ ] Write tests for communication-source bypass.
- [ ] Verify RED.
- [ ] Implement pure resolution logic.
- [ ] Verify GREEN.
- [ ] Commit.

---

## Task 16: Expose Typed Tauri Command Boundary

**Files:**
- Create: `tauri/app/dto.rs`
- Create: `tauri/app/state.rs`
- Update: `tauri/app/commands.rs`
- Create: `ui/lib/tauri.ts`
- Create: `ui/lib/tauri.test.ts`

**Allowed command surface initially:**

```text
get_app_state
set_master_enabled
set_processing_mode
set_engine
set_vocal_level
set_quality_preference
list_audio_sources
list_audio_outputs
set_app_override
set_output_route
```

No arbitrary command execution.

No arbitrary path access.

No generic shell.

- [ ] Write Rust tests for DTO validation.
- [ ] Write TS tests for invoke wrappers using injected invoke function, not global Tauri.
- [ ] Verify RED.
- [ ] Implement narrow commands.
- [ ] Verify GREEN.
- [ ] Update Tauri capability file to only expose needed commands.
- [ ] Commit.

---

## Task 17: Build Functional UI Screens Against Local State/Typed Boundary

**Files by feature:**
- `ui/features/home/*`
- `ui/features/apps/*`
- `ui/features/routing/*`
- `ui/features/engine/*`
- `ui/features/settings/*`

**Home must include:**
- Master processing toggle.
- All Output / Per-App selector.
- Engine selector.
- Vocals slider.
- Latency ↔ Quality slider.
- Current estimated latency/CPU mode.
- Physical/Virtual output summary.

**Apps:**
- source list;
- per-app toggle;
- global inheritance;
- optional vocal/quality/engine override.

**Routing:**
- physical output;
- virtual output;
- simultaneous output.

**Engine:**
- Classic DSP description;
- optional AI availability;
- measured status.

**Settings:**
- theme;
- language;
- hotkey placeholder only when native hotkey backend exists;
- local privacy statement;
- diagnostics export only when implemented.

For each component:
- [ ] write rendering/interaction test first;
- [ ] verify RED;
- [ ] implement minimal UI;
- [ ] verify GREEN;
- [ ] run coverage;
- [ ] run LOC gate.

**Responsive QA sizes:**
- 320×568
- 390×844
- 768×1024
- 1024×768
- 1440×900

**Commit after each feature screen rather than one giant commit.**

---

## Task 18: Windows Standard Audio Adapter

**Files:**
- `tauri/platform/windows/mod.rs`
- `tauri/platform/windows/session.rs`
- `tauri/platform/windows/capture.rs`
- `tauri/platform/windows/output.rs`
- `tauri/platform/windows/tests/*`
- `docs/specs/platform/windows.md`

**Goal:** supported public Windows APIs for device/session discovery, loopback/media capture where appropriate, output routing, and per-session metadata.

**Rules:**
- FFI/native calls contained in this module.
- Safe wrappers exposed to shared routing layer.
- No unsafe code outside the platform module.
- Restore prior routing on clean shutdown when Voxveil changed routing.

- [ ] Write trait-contract tests with fake backend.
- [ ] Implement session enumeration.
- [ ] Implement capture adapter.
- [ ] Implement output device adapter.
- [ ] Add device-disconnect recovery tests.
- [ ] Add Windows CI compile/build job.
- [ ] Update spec with actual limitations measured.
- [ ] Commit.

---

## Task 19: Linux Standard Audio Adapter

**Files:**
- `tauri/platform/linux/mod.rs`
- `tauri/platform/linux/pipewire.rs`
- `tauri/platform/linux/streams.rs`
- `tauri/platform/linux/tests/*`
- `docs/specs/platform/linux.md`

**Primary backend:** PipeWire.

- [ ] Write routing trait-contract tests.
- [ ] Implement node/stream enumeration.
- [ ] Implement capture/output graph creation.
- [ ] Implement per-stream routing.
- [ ] Implement clean graph restoration.
- [ ] Add Linux CI integration smoke test under a controlled virtual PipeWire environment where practical.
- [ ] Commit.

---

## Task 20: macOS Standard Audio Adapter

**Files:**
- `tauri/platform/macos/mod.rs`
- `tauri/platform/macos/core_audio.rs`
- `tauri/platform/macos/taps.rs`
- `tauri/platform/macos/output.rs`
- `tauri/platform/macos/tests/*`
- `docs/specs/platform/macos.md`

- [ ] Write trait-contract tests.
- [ ] Implement device enumeration.
- [ ] Implement permitted outgoing-audio capture/tap path.
- [ ] Implement output routing.
- [ ] Add permission/error-state mapping.
- [ ] Add macOS CI compile/build.
- [ ] Commit.

---

## Task 21: Desktop Virtual Processed Output

**Files:**
- platform-specific virtual-output modules under Windows/Linux/macOS
- `tauri/routing/virtual_output.rs`
- tests per platform
- release/package specs

**Output modes:**

```text
Physical only
Virtual only
Physical + Virtual
```

**Critical rule:** process once, fan out processed PCM. Never run separator/DSP twice solely because two outputs are active.

- [ ] Write shared fan-out tests.
- [ ] Verify RED.
- [ ] Implement single-pipeline fan-out.
- [ ] Verify GREEN.
- [ ] Implement platform virtual endpoint adapters.
- [ ] Verify OBS/recording app visibility manually on each desktop OS.
- [ ] Commit per platform.

---

## Task 22: Hotkeys, Tray/Menu-Bar, and Quick Controls

**Files:**
- `tauri/app/hotkey.rs`
- `tauri/app/tray.rs`
- `ui/components/QuickControls.tsx`
- platform-specific setup as required

**Controls:**
- master bypass;
- quick vocal amount;
- current engine/status;
- open Voxveil.

- [ ] Audit exact official Tauri plugin(s) if needed before adding.
- [ ] Do not add a general shell plugin.
- [ ] Write state transition tests.
- [ ] Implement.
- [ ] Verify on Windows/Linux/macOS.
- [ ] Commit.

---

## Task 23: AI Model API Without Shipping a Model

**Files:**
- `crates/voxveil-model-api/Cargo.toml`
- `crates/voxveil-model-api/src/lib.rs`
- `crates/voxveil-model-api/src/metadata.rs`
- `crates/voxveil-model-api/src/license.rs`
- `crates/voxveil-model-api/src/license_test.rs`
- `docs/specs/audio/model-interface.md`

**Model metadata must contain:**

```text
id
display_name
version
model_sha256
source
code_license
weights_license
commercial_use_allowed
redistribution_allowed
modification_allowed
required_attribution
sample_rates
channels
latency_class
quality_class
backend
```

- [ ] Write tests rejecting missing/ambiguous license fields.
- [ ] Write tests rejecting `commercial_use_allowed=false`.
- [ ] Write tests rejecting `redistribution_allowed=false` for bundled models.
- [ ] Verify RED.
- [ ] Implement metadata validator.
- [ ] Verify GREEN.
- [ ] Do not integrate ONNX Runtime or model files yet.
- [ ] Commit.

---

## Task 24: AI Candidate Audit and Prototype Gate

**Documents only until license clears:**
- `docs/specs/audio/ai-model-audit.md`

For every candidate:
- exact repository;
- exact code license;
- exact checkpoint file;
- checkpoint hash;
- exact checkpoint license;
- commercial-use clause;
- redistribution clause;
- modification clause;
- attribution;
- training dataset concerns if relevant;
- model format;
- runtime requirements;
- measured CPU/GPU/memory;
- measured latency;
- quality notes.

**Hard decision rule:**
If any checkpoint term is unclear, status = `REJECTED_FOR_SHIPPING`.

Prototype-only use does not imply shipping approval.

- [ ] Audit candidate.
- [ ] Record evidence.
- [ ] Have license quality gate parse approved model manifest.
- [ ] Only after approval, create a separate implementation task to add the chosen inference runtime.
- [ ] Keep Classic DSP as default fallback.

---

## Task 25: Auto/Adaptive Runtime Controller

**Files:**
- `tauri/realtime/adaptive.rs`
- `tauri/realtime/adaptive_test.rs`
- `tauri/realtime/metrics.rs`

**Inputs:**
- processing time / audio period;
- underrun count;
- CPU capability;
- optional GPU/NPU capability;
- battery state;
- thermal state;
- user quality ceiling.

**Outputs:**
- selected engine/tier;
- buffer/window size;
- safe fallback action.

- [ ] Write deterministic state-machine tests.
- [ ] Verify RED.
- [ ] Implement without OS telemetry/network.
- [ ] Verify GREEN.
- [ ] Add hysteresis to avoid oscillation.
- [ ] Test downgrade and recovery.
- [ ] Commit.

---

## Task 26: Android Standard Edition

**Files:**
- `tauri/platform/android/*`
- generated Tauri Android project as required
- `docs/specs/platform/android.md`

**Scope:**
- use only public permitted capture paths;
- respect applications/audio categories that cannot be captured;
- communicate limitation in UI;
- local DSP core shared from Rust.

- [ ] Initialize Android target.
- [ ] Keep generated files in documented generated allowlist.
- [ ] Write Rust-side contract tests.
- [ ] Implement public capture/output bridge.
- [ ] Add Android instrumentation smoke test where possible.
- [ ] Build APK/AAB in manual-build workflow.
- [ ] Commit.

---

## Task 27: Android Pro System Edition

**Separate privileged artifact.**

Do not mix privileged behavior into Standard package.

**Files:**
- dedicated module/package location documented in Android platform spec;
- separate Tauri build flavor/config;
- separate package identifier if required.

**Security rules:**
- least privilege;
- explicit install warning;
- no hidden persistence;
- no remote command channel;
- no telemetry;
- rollback/uninstall restores audio state.

- [ ] Create threat model.
- [ ] Define exact supported rooted/privileged mechanism.
- [ ] Write integration tests around bridge protocol.
- [ ] Sign privileged companion/module separately.
- [ ] Package as distinct artifact.
- [ ] Commit only after security review.

---

## Task 28: iOS Standard Edition

**Files:**
- `tauri/platform/ios/*`
- generated Tauri iOS project as required
- `docs/specs/platform/ios.md`

**Scope:** only public iOS APIs and app-permitted audio paths.

- [ ] Initialize iOS target on macOS.
- [ ] Write shared Rust contract tests.
- [ ] Implement supported capture/output bridge.
- [ ] Ensure UI truthfully reports unavailable system-wide features.
- [ ] Add simulator/device smoke testing.
- [ ] Build unsigned/signed artifact according to CI secret availability.
- [ ] Commit.

---

## Task 29: iOS Pro System Edition

This is a separately distributed privileged/jailbreak-oriented variant.

- [ ] Write threat model before code.
- [ ] Define supported privileged environment and versions.
- [ ] Keep privileged bridge small and auditable.
- [ ] No runtime network listener.
- [ ] No remote commands.
- [ ] Separate package identity/artifact.
- [ ] Add rollback behavior.
- [ ] Do not advertise normal App Store compatibility for this artifact.
- [ ] Commit only after security review.

---

# 8. CI and Quality Automation

## Task 30: GitHub Actions CI

**File:** `.github/workflows/ci.yml`

**Triggers:**

```yaml
on:
  push:
    branches: [master]
  pull_request:
    branches: [master]
```

**Permissions:**

```yaml
permissions:
  contents: read
```

Raise permissions only in a specific job that proves it needs them.

### Jobs

1. `js-quality`
   - checkout pinned action;
   - exact pnpm version from `packageManager`;
   - frozen install;
   - Node tests for quality scripts;
   - TypeScript typecheck;
   - Vitest;
   - frontend coverage ≥85%;
   - LOC gate;
   - i18n gate;
   - network-surface gate.

2. `rust-quality`
   - rustfmt check;
   - clippy `--all-targets --all-features -- -D warnings`;
   - workspace tests;
   - cargo-llvm-cov ≥85%;
   - LOC gate.

3. `dependency-security`
   - cargo-deny;
   - cargo-audit;
   - pnpm audit;
   - lockfile consistency;
   - no exotic dependencies;
   - license report.

4. `build-smoke`
   - matrix: Windows, Linux, macOS;
   - compile Tauri shell;
   - later extend to Android/iOS build validation.

5. `ci-required`
   - one final aggregate required status.

**PR cache security:**
Untrusted PR jobs must not write caches that trusted release jobs consume.

- [ ] Create workflow.
- [ ] Pin third-party actions to immutable commit SHA.
- [ ] Verify no secrets are exposed to PR jobs.
- [ ] Verify branch protection can require `ci-required`.
- [ ] Commit.

---

## Task 31: PR Checklist

**File:** `.github/pull_request_template.md`

Required checklist:

```markdown
- [ ] Tests added/updated.
- [ ] Coverage remains >= 85%.
- [ ] LOC limits pass.
- [ ] No unnecessary dependency added.
- [ ] New dependency license verified.
- [ ] New dependency source/build scripts reviewed.
- [ ] No telemetry/network behavior introduced.
- [ ] i18n updated for user-facing text.
- [ ] Light theme checked.
- [ ] Dark theme checked.
- [ ] Responsive layout checked.
- [ ] Accessibility/focus checked.
- [ ] Relevant specs updated.
- [ ] Security implications reviewed.
```

---

## Task 32: Manual Build Workflow

**File:** `.github/workflows/manual-build.yml`

This follows the proven OmniTerm-style pattern:
- `workflow_dispatch`;
- selectable git ref;
- optional version/build-only behavior;
- tests before package jobs;
- package verification;
- checksums;
- artifact upload;
- no release publication in build-only mode.

### Inputs

```yaml
workflow_dispatch:
  inputs:
    git_ref:
      description: "Branch, tag, or commit SHA; blank uses selected UI ref"
      required: false
      type: string
    version:
      description: "auto, exact vX.Y.Z, or blank for build-only"
      required: false
      default: ""
      type: string
    platform:
      description: "Platform"
      required: true
      default: all
      type: choice
      options: [all, windows, linux, macos, android, ios]
    edition:
      description: "Edition"
      required: true
      default: all
      type: choice
      options: [all, standard, pro-system]
```

### Gates

```text
test-gate
rust-test-gate
security-gate
resolve-build-version
        ↓
build matrix
        ↓
artifact verification
        ↓
SHA-256 checksums
        ↓
upload artifacts
```

### Artifact naming

```text
Voxveil-Windows-x64-Standard
Voxveil-Windows-x64-ProSystem
Voxveil-Linux-x64-Standard
Voxveil-Linux-x64-ProSystem
Voxveil-macOS-arm64-Standard
Voxveil-macOS-arm64-ProSystem
Voxveil-Android-arm64-Standard
Voxveil-Android-arm64-ProSystem
Voxveil-iOS-arm64-Standard
Voxveil-iOS-arm64-ProSystem
```

Architectures can be expanded later via a spec change; do not silently add unsupported architectures.

### Manual build publication rule

Default manual build: upload workflow artifacts only.

If the version input resolves to an explicit release mode in a later approved workflow rule, publication must still be separated behind a job with `contents: write`. Initial Voxveil implementation keeps `manual-build.yml` build-only to reduce release risk.

- [ ] Build workflow.
- [ ] Verify input validation.
- [ ] Verify invalid semantic versions fail.
- [ ] Verify missing expected artifacts fail.
- [ ] Verify checksums generated.
- [ ] Verify no release or signing secrets are available to build-only jobs.
- [ ] Commit.

---

## Task 33: Tag Release Workflow

**File:** `.github/workflows/release.yml`

**Trigger:**

```yaml
on:
  push:
    tags:
      - "v*.*.*"
```

First job must additionally validate strict SemVer:

```text
^v[0-9]+\.[0-9]+\.[0-9]+$
```

Examples accepted:
- `v0.1.0`
- `v1.0.0`
- `v2.4.17`

Examples rejected:
- `v1`
- `v1.2`
- `release-1.2.3`
- `v1.2.3-beta.1` unless a future spec explicitly adds prerelease support.

### Release flow

```text
validate tag/version consistency
        ↓
full CI gates
        ↓
build platform/edition matrix
        ↓
verify artifacts
        ↓
generate SHA-256 checksums
        ↓
generate SBOM
        ↓
generate dependency/license report
        ↓
sign configured artifacts
        ↓
create GitHub Release
        ↓
attach artifacts
```

### Version consistency

The tag version must match:
- root `package.json`;
- `tauri/tauri.conf.json`;
- `tauri/Cargo.toml`;
- workspace package versions where applicable.

Mismatch = fail before build.

### Signing

Signing keys/secrets:
- only available to protected release environment/job;
- never available to PR jobs;
- never printed;
- platform signing is skipped only when a release spec explicitly allows unsigned artifacts for that platform.

- [ ] Implement release workflow.
- [ ] Add release script unit tests for version consistency.
- [ ] Test workflow with a dry-run branch/manual harness before first real tag.
- [ ] Commit.

---

# 9. Coverage and Test Strategy

## Frontend

Use Vitest with V8 coverage.

Required metrics:

```text
lines      >= 85
branches   >= 85
functions  >= 85
statements >= 85
```

Additionally, feature directories should not be hidden by one highly tested module. CI should publish per-file coverage for review.

## Rust

Use:

```bash
cargo llvm-cov --workspace --all-features --fail-under-lines 85
```

Where practical, also generate LCOV for CI artifacts.

Platform FFI code that cannot be executed on generic CI still requires:
- compile checks on its native OS;
- safe-wrapper tests;
- fake-backend behavior tests;
- manual/integration checklist.

## Audio tests

No copyrighted commercial music fixtures in repository.

Prefer generated synthetic signals:
- centered sine;
- side-only sine;
- impulse;
- white/pink deterministic noise;
- two-tone mixtures;
- amplitude ramps;
- phase-shifted stereo;
- generated speech-like harmonic envelopes if needed.

Any external audio fixture must have an explicit redistribution license recorded beside the file.

---

# 10. Performance Gates

Before declaring audio core complete, create benchmarks for:

- 48 kHz stereo;
- 128, 256, 512, 1024 frame blocks;
- Classic DSP mid/side;
- STFT processing;
- buffer enqueue/dequeue;
- fan-out physical + virtual.

Record:
- median processing time;
- p95;
- p99;
- allocations per steady-state block;
- estimated algorithmic latency.

Hard realtime callback goal:
- no heap allocation after warmup;
- no blocking lock;
- no disk/network;
- no model load.

Performance regression threshold should be added only after a stable baseline exists; do not invent a percentage before baseline measurement.

---

# 11. Error and Recovery Contract

## Audio device disappears

```text
detect
→ stop route safely
→ select configured fallback if available
→ otherwise bypass
→ notify UI locally
```

No crash.

## DSP overload

```text
quality tier down
→ simpler engine
→ Classic DSP
→ bypass
```

No broken looping audio.

## Config corruption

- preserve corrupt file with `.corrupt-<timestamp>` suffix;
- load safe defaults;
- show local warning;
- do not upload file.

## AI model corrupt/hash mismatch

- refuse load;
- Classic DSP remains available;
- display model error locally.

## Privileged helper unavailable

- Pro feature reports unavailable;
- Standard functionality remains usable where possible;
- no repeated privilege prompt loop.

---

# 12. Logging and Diagnostics

No telemetry.

Default production log level should be minimal.

Logs must:
- stay local;
- avoid raw audio samples;
- avoid full user file paths where unnecessary;
- avoid secrets/tokens;
- rotate/limit size if file logging is later enabled.

Diagnostics export is user-initiated only and must show what will be exported.

Do not add OpenTelemetry exporters, Sentry, analytics, or remote logging.

---

# 13. Local Persistence

Store only:
- theme preference;
- language;
- processing defaults;
- per-app rules;
- output preference;
- hotkey;
- locally installed model metadata;
- user-approved diagnostics preferences.

Do not persist raw audio.

Configuration format should be versioned.

Migration tests required before schema changes.

---

# 14. Model Shipping Rule

A model cannot be present in a release artifact unless CI can find a checked-in approved metadata record containing:
- SHA-256;
- source;
- license identifiers/text references;
- commercial permission;
- redistribution permission;
- modification permission;
- attribution data.

Release job must verify the model file hash against this metadata.

No "community model" is accepted by reputation alone.

---

# 15. Completion Gates by Milestone

## Milestone A — Secure foundation

Complete when:
- specs written;
- pnpm/Rust workspaces boot;
- LOC gate active;
- coverage gates active;
- UI shell + light/dark + i18n work;
- Tauri shell runs;
- no network capability;
- CI active.

## Milestone B — Useful zero-AI desktop prototype

Complete when:
- shared audio core works;
- Classic DSP works;
- All Output routing works on first desktop OS;
- physical output works;
- user can change vocal suppression and quality;
- UI connected to Rust;
- ≥85% coverage.

## Milestone C — Desktop product

Complete when:
- Windows/Linux/macOS adapters;
- per-app routing;
- virtual output;
- tray/hotkeys;
- failure recovery;
- manual build matrix.

## Milestone D — Optional AI

Complete when:
- exact checkpoint passes legal/license gate;
- model hash/package metadata exists;
- local inference integrates;
- Auto fallback works;
- Classic DSP remains independently usable.

## Milestone E — Mobile Standard

Complete when:
- Android/iOS public-API limitations documented;
- permitted audio processing works;
- mobile-responsive UI fully tested;
- mobile build artifacts automated.

## Milestone F — Pro System

Complete separately for each OS privileged mechanism.
No platform is marked complete merely because shared UI builds.

## Milestone G — Release-ready

Complete when:
- full CI;
- release workflow;
- signed artifacts where configured;
- SBOM;
- checksums;
- license report;
- no unresolved high/critical advisories;
- final privacy/network audit;
- docs updated.

---

# 16. Implementation Working Rules

1. Work on a feature branch/worktree, never directly on `master`.
2. Execute one task at a time.
3. TDD for behavior.
4. Run task-specific tests immediately.
5. Run LOC gate after every source-heavy task.
6. Run coverage before every task commit that changes executable behavior.
7. Keep commits small and reviewable.
8. Do not add a package because implementation is easier; first prove it is needed.
9. Do not move DSP into React.
10. Do not put OS-specific code into shared crates.
11. Do not put unsafe native FFI into domain/routing/DSP crates.
12. Do not fake support for a platform that only compiles.
13. Do not market "perfect vocal removal".
14. Do not ship an AI weight with ambiguous license.
15. Do not enable a broad Tauri permission to solve a narrow command need.
16. Do not use remote CDN assets.
17. Do not add hidden update/network calls.
18. Keep all locale resources packaged locally.
19. Do not exceed LOC limits; split by responsibility.
20. Every release-producing workflow must depend on all quality/security gates.

---

# 17. Verification Before Each Merge

Run, as applicable:

```bash
pnpm install --frozen-lockfile
node --test scripts/quality/*.test.mjs
pnpm quality:loc
pnpm quality:i18n
pnpm quality:network
pnpm --filter @voxveil/ui test -- --run
pnpm --filter @voxveil/ui coverage

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features --fail-under-lines 85
cargo deny check
cargo audit
```

Then build the affected native platform.

For UI changes:
- Light theme;
- Dark theme;
- keyboard;
- reduced motion;
- 320 px width;
- 390 px width;
- tablet;
- desktop;
- long French/Spanish/Vietnamese labels;
- CJK labels.

---

# 18. First Implementation Slice

Implementation should **not** start by attempting system-wide audio interception on all five OSes.

The first executable slice is:

```text
Specs
→ pnpm/Rust workspaces
→ security/LOC/coverage gates
→ responsive Editorial Monochrome UI shell
→ seven-language local i18n
→ custom /tauri shell
→ typed shared Rust settings/domain
→ Classic DSP unit-tested core
→ fake routing backend
→ UI connected to fake/local processing state
→ CI + manual-build skeleton
```

This proves:
- architecture;
- custom folder structure;
- dependency policy;
- Tauri boundary;
- responsiveness;
- i18n;
- DSP design;
- quality gates.

Only then start native audio routing.

---

# 19. References Used to Validate This Plan

Primary/reference documentation consulted before implementation:

- Tauri 2 Project Structure: https://v2.tauri.app/start/project-structure/
- Tauri 2 Configuration Files: https://v2.tauri.app/develop/configuration-files/
- Tauri + Vite: https://v2.tauri.app/start/frontend/vite/
- Cargo target path configuration: https://doc.rust-lang.org/cargo/reference/cargo-targets.html
- pnpm supply-chain guidance: https://pnpm.io/supply-chain-security
- pnpm CI guidance: https://pnpm.io/continuous-integration
- pnpm installation/version pinning: https://pnpm.io/installation
- GitHub Actions workflow syntax: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
- RustFFT source/license: https://github.com/ejmahler/RustFFT
- Serde source/license: https://github.com/serde-rs/serde
- Tauri source/license: https://github.com/tauri-apps/tauri
- pnpm source/license: https://github.com/pnpm/pnpm

---

# 20. Plan Self-Review Result

## Spec coverage

Covered:
- Voxveil naming;
- all five platforms;
- Standard and Pro System;
- local-first/offline;
- non-AI Classic DSP;
- optional AI with strict checkpoint licensing;
- vocal control;
- latency/quality control;
- all-output/per-app;
- calls/VoIP bypass;
- physical/virtual output;
- Tauri custom `/ui` + `/tauri` layout;
- no `/tauri/src`;
- Editorial Monochrome light/dark;
- responsive UI;
- seven languages;
- LOC limits;
- ≥85% coverage;
- dependency/supply-chain gates;
- master/PR CI;
- manual build;
- tag release.

## Ambiguity resolution

- "No supply-chain risk" is implemented as a strict risk-minimization/audit policy; absolute zero risk cannot be guaranteed.
- DSP slider semantics are suppression strength, not a fictional isolated vocal stem.
- Manual build is build-only by default; release publication belongs to the tag release workflow.
- Privileged mobile editions remain separate artifacts.
- Exact AI runtime/model is intentionally not selected before exact checkpoint licensing is proven.

## Placeholder scan

No required product behavior is left as `TBD` or `TODO`.

## Type consistency

Core shared type and trait names are defined once in this plan and reused consistently.

---

# 21. Execution Rule

After this plan is approved, implementation begins with **Task 1** and proceeds in order.

The first implementation session should complete Milestone A before moving to native audio platform adapters.
