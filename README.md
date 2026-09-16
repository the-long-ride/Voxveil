# Voxveil

Voxveil is a local-first, cross-platform real-time vocal-reduction application. It is designed around a shared Rust audio core, native platform audio adapters, and a responsive Tauri/React interface.

## Current implementation

This source tree contains:

- responsive Editorial Monochrome UI with light/dark/system themes;
- English, Vietnamese, Chinese, Korean, Japanese, Spanish, and French bundles;
- local-only state and a narrow typed Tauri command bridge;
- Classic DSP adaptive stereo-center suppression with user-selectable **Music preservation** and **Balanced** profiles and no AI dependency;
- stem-agnostic optional AI interface with no model bundled;
- global/per-app routing policy and communication-audio bypass rules;
- fixed-capacity audio buffering and runtime degradation primitives;
- Standard/Pro System edition metadata and platform capability contracts;
- dependency, LOC, i18n, network-surface, coverage, license, repository-hygiene, and workflow-free policy gates;
- a Windows x64 componentized Audio Processing Object implementation under `native/windows/apo`;
- a render-only first-party Windows virtual-driver source/package boundary derived from pinned Microsoft SysVAD source;
- workflow-free local/manual Windows application + native component staging via `npm run build:windows`.

### Windows system audio

Windows interception uses explicit backend precedence:

1. **Endpoint-scoped loaded Voxveil APO** — `VoxveilApo.dll` must have a real non-discovery AudioDG instance and `install-state.json` must show that Voxveil installed/bound the APO on the current Windows default render endpoint. A stale loaded instance on another endpoint does not win backend selection.
2. **Voxveil virtual endpoint relay** — otherwise, if a verified Voxveil virtual render endpoint is installed, it is preferred as the relay source.
3. **Standard VB-CABLE relay** — otherwise, standard VB-Audio `CABLE Input` can provide the same relay path.

When Windows changes the default output away from an APO-installed physical endpoint while an old AudioDG graph remains alive, Voxveil does not treat the process-wide `loaded>0` counter as proof that the APO covers the new route. The APO app gate is disabled before virtual-relay processing so a stale instance cannot suppress relay fallback or double-process the relay destination.

The relay path is:

```text
Windows applications
  -> controlled virtual render endpoint
  -> WASAPI loopback capture
  -> Voxveil Rust DSP / optional AI
  -> selected physical output
```

The Classic DSP relay retains a persistent 512-frame / 128-hop spectral-center processor. It estimates center likelihood from stereo magnitude balance and phase coherence, protects bass/high-frequency detail and transients, and uses non-zero suppression floors so maximum vocal reduction does not hard-delete the center. **Music preservation** is the default; **Balanced** trades more centered-instrument attenuation for stronger vocal reduction. See `docs/specs/audio/classic-dsp.md`.

The selected physical output is stored by endpoint ID, virtual endpoints are excluded from the physical-output list, and Voxveil rejects a route that would render back into the interception endpoint. `ready` is reported only while the selected interception strategy is actually active. A missing supported virtual endpoint is `component-required`; installed-but-not-default routing or a missing safe sink is `routing-required`; relay startup/runtime failures are `faulted`.

Voxveil does **not** bundle, redistribute, silently download, or silently install VB-CABLE. When it is needed, the UI can open the official VB-Audio page and Windows Sound settings. The relay intentionally does not mutate Windows defaults through undocumented `PolicyConfig` interfaces.

The first-party virtual driver exposes one render endpoint (`Voxveil Input`), no capture/microphone endpoint, and a stereo 48 kHz float32 path compatible with the same user-mode relay. The local WDK project does not test-sign itself. Repository tooling prepares x64/Arm64 packages and submission CABs, verifies returned Microsoft-signed packages, and keeps pilot versus retail staging separate. Microsoft attestation signing is treated as a pilot/direct-validation path, not as Windows Certified retail qualification.

The componentized APO uses the Windows 11 CAPX model while retaining older initialization compatibility:

- `native/windows/package/VoxveilApo.inf` installs the APO software component and COM/audio-engine registration;
- `native/windows/package/VoxveilApoExtension.inf.template` supports signed endpoint-interface CAPX binding;
- the Windows 11 CAPX context is `{63E268CE-4CBC-48E0-BEB6-55103316F477}`;
- the controllable Voxveil vocal-suppression effect is `{B9FD554E-8F72-4B20-9AB1-13F8E8BFDD02}`;
- discovery-only `IAudioSystemEffects3` instances increment `capx` diagnostics but never the `loaded` real-processing counter;
- Windows system-effect state and the Voxveil app master state are independent processing gates;
- production Windows 11 packages use the context-qualified CAPX association under `FX\0\{context}`;
- direct `voxveil-control.exe attach-effects` root-`FX\0` mutation is retained only for explicit development/legacy flows and is not the production CAPX path;
- `scripts/windows/probe-apo-capx.ps1` reads `loaded`, `capx`, `system-effect`, and heartbeat state without mutating the system;
- APO uninstall is scoped to the APO/Extension INF names recorded by its installer and does not fall back to deleting every Voxveil driver, so the independent first-party virtual driver is not removed accidentally.

The repository never represents unsigned or test-signed kernel/audio packages as retail-ready. Certificate purchase, identity vetting, Partner Center submission, Microsoft signing, HLK/WHCP qualification, and release validation are external release operations tied to the exact artifacts being shipped.

Build the Windows development package from an x64 Developer PowerShell with Visual Studio C++ Build Tools and the Windows Driver Kit installed:

```powershell
npm run build:windows
```

The staged output is written to `dist/windows-x64/Voxveil` by default. The `npm run build:windows` entrypoint performs a clean dependency install, runs `cargo test --workspace`, the UI/Node test suite, TypeScript typechecking, and the full repository quality gate, then invokes the Windows packager with only dependency installation skipped. The packager reruns its focused Windows-audio and quality checks and builds/executes the native APO policy tests before compiling the control/APO projects, virtual-device helper, and Tauri executable and writing SHA-256 checksums.

For release-candidate staging, signed native packages are opt-in inputs and are verified before they replace development artifacts in the staged desktop package. Set only the inputs that are available for the release being assembled:

```powershell
$env:VOXVEIL_SIGNED_APO_DIR = 'C:\path\to\microsoft-signed-apo-package'
$env:VOXVEIL_SIGNED_DRIVER_DIR = 'C:\path\to\microsoft-signed-virtual-driver-package'
$env:VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL = 'Retail'
npm run build:windows
```

`VOXVEIL_SIGNED_APO_DIR` must contain the matching production `VoxveilApo.inf`, `VoxveilApo.dll`, `VoxveilApo.cat`, `VoxveilApoExtension.inf`, and `VoxveilApoExtension.cat`. The build invokes `verify-signed-apo-package.ps1` through the staging wrapper, checks the fixed CAPX/Extension identities, catalog membership, x64 PE architecture, Microsoft catalog signatures, and the APO DLL PETrust/Authenticode signature before copying those files into `system-audio`. The stager also writes `apo-verification.json`; endpoint discovery requires that marker, and the elevated installer re-hashes all five signed artifacts against it immediately before PnP installation. If this variable is absent, the locally built APO artifacts remain development artifacts and must not be treated as a production-signed APO package.

`VOXVEIL_SIGNED_DRIVER_DIR` is independently validated by the virtual-driver staging gate. Retail is the default signed-driver release channel and requires matching external release evidence; pilot use must be selected explicitly with `VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL=Pilot`.

Real Windows audio behavior still requires the hardware/OS verification matrices under `docs/testing/` before release. Retail claims additionally require the release gates under `docs/release/`.

## Privacy and networking

Voxveil processing is designed to work with no network connection. The application has no telemetry, analytics, remote fonts, cloud audio processing, or generic Tauri HTTP capability. Network-dependent developer operations such as package installation, advisory lookup, driver-source import, signing submission, and release publishing are build-time/repository operations rather than app runtime behavior.

### Optional AI model

Voxveil does not bundle AI weights. The Engine screen can install a reviewed model only after explicit user consent. Downloads are pinned to an approved source revision, stored under Voxveil's local application-data directory, and SHA-256 verified before installation. The model can be removed from the same screen. See `docs/specs/audio/ai-model-delivery.md`.
