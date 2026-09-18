# Windows Audio Platform Specification

## Scope

Windows Standard and Pro System editions share the Rust processing graph. The Windows platform layer owns audio interception, endpoint discovery, output-device changes, and the optional privileged native components.

## Interception rule

WASAPI loopback is only a capture mechanism. Voxveil must not claim system-wide processing when it merely loopback-captures a physical endpoint while the original stream still reaches that same physical endpoint unchanged.

A Windows build may report `backendStatus = ready` only when audio is routed through a controlled interception strategy that replaces the original playback path or when a loaded in-process APO is actually modifying the current default render graph.

## Runtime backend precedence

The runtime chooses Windows interception in this order:

1. a Voxveil APO that has at least one real non-discovery AudioDG instance **and** whose persisted installed endpoint matches the current default render endpoint;
2. a verified Voxveil virtual render endpoint using the user-mode WASAPI relay;
3. a standard VB-CABLE `CABLE Input` render endpoint using the same relay.

A process-wide `loaded>0` count alone does not give the APO priority. If an APO remains loaded on another/stale physical endpoint after Windows changes the default output, Voxveil disables its app-processing gate before allowing a virtual relay to process the current route. This prevents stale APO presence from suppressing relay fallback or causing double Voxveil processing.

When both supported virtual endpoints exist, the first-party Voxveil endpoint wins over VB-CABLE.

## Virtual-endpoint relay

The shared relay path is:

```text
Windows applications
  -> controlled virtual render endpoint
  -> WASAPI loopback capture
  -> Voxveil Rust DSP / optional AI
  -> selected physical output
```

For VB-CABLE, the controlled render endpoint is standard `CABLE Input`. Voxveil does not redistribute or silently install VB-CABLE. The application may open only the official VB-Audio download page and Windows Sound settings. The relay requires the selected virtual endpoint to be the ordinary Windows default render endpoint; it does not use undocumented `PolicyConfig` APIs to mutate defaults.

For the first-party path, the controlled endpoint is `Voxveil Input` from the render-only Voxveil virtual driver. The same relay implementation is reused rather than duplicating audio capture/DSP/render code.

The physical destination is selected by stable endpoint ID. Known virtual endpoints are excluded, and the source and sink IDs must be distinct to prevent feedback recursion.

## Readiness contract

- `component-required`: no endpoint-scoped loaded APO and no supported interception component such as the Voxveil virtual endpoint or standard VB-CABLE is available for the current route.
- `routing-required`: the supported virtual endpoint exists but is not the ordinary Windows default render endpoint, or no safe physical destination can be selected.
- `faulted`: interception was selected but enumeration, APO status/install-state validation, capture, DSP, render, or relay startup/runtime failed.
- `ready`: either (a) the APO has a real non-discovery instance and its persisted installed endpoint is the current default render endpoint, or (b) the virtual relay worker has successfully started capture and physical rendering.
- `unsupported`: the platform/build cannot provide the requested interception strategy.

The master Processing switch must not remain enabled after a backend becomes non-ready.

## APO path

The componentized `VoxveilApo.dll` is the highest-precedence strategy only when its endpoint coverage is proven for the current default render route.

On Windows 11 the APO implements `IAudioSystemEffects3` and the CAPX settings/effects-discovery model. Its fixed CAPX property context is `{63E268CE-4CBC-48E0-BEB6-55103316F477}` and its controllable vocal-suppression effect is `{B9FD554E-8F72-4B20-9AB1-13F8E8BFDD02}`.

The shared control mapping distinguishes real processing instances from CAPX discovery:

- `loaded` counts only successfully initialized non-discovery processing instances;
- `capx` counts successful v3/CAPX instances, including discovery-only instances;
- `system-effect` is the Windows effect-state gate;
- `loaded` proves that a real processing instance exists, while `install-state.json` supplies the endpoint identity required to prove that the instance is relevant to the current default route.

Production Windows 11 endpoint packages use the signed CAPX context association under `FX\0\{context}`. Direct `voxveil-control attach-effects` mutation of root `FX\0` properties is retained only for explicit development/legacy flows and is not a production CAPX path.

A production APO package is considered installable only after it has been staged through the signed-APO verifier. The staged `apo-verification.json` records hashes and fixed identities for the five signed APO/Extension artifacts. The elevated installer recomputes those hashes immediately before PnP installation; a copied, modified, or incomplete lookalike package is rejected.

`install-state.json` records the endpoint that was selected during APO installation and only the APO/Extension INF package names owned by that installer. The current control ABI reports a process-wide `loaded` instance count, not endpoint-scoped loaded-instance identities, so one install-state file manages exactly one endpoint at a time. A different endpoint must be rejected before PnP mutation while any endpoint-scoped install state remains, including partial/non-ready state; uninstall the managed APO state/packages before switching the managed endpoint. The UI exposes explicit per-endpoint installation rather than a bulk install action for the same reason. Development/TestSign state also owns the exact generated certificate thumbprint, reuses it across repair/reboot retries, and removes it from the machine/trust stores only after scoped package cleanup. Legacy runtime-interface state separately records `legacyRuntimeAttached`; successful attach/detach transitions are checkpointed so early TestSign failures do not trigger a false detach and retries do not repeat a completed detach. Final uninstall persists an `audioServiceRestartRequired` checkpoint before the last `AudioSrv` restart so a failed restart is resumable and blocks a new install until cleanup completes. If `install-state.json` exists but the Voxveil control component is unavailable or its load state cannot be verified, runtime status is `faulted` and must not proceed to relay fallback. APO uninstall removes only those recorded packages. It must never fall back to deleting every PnP driver whose provider is Voxveil because the Tier 2 virtual driver is an independent component and may coexist on the machine.

Older Windows initialization remains source-compatible through `APOInitSystemEffects`/`APOInitSystemEffects2`. Windows 10 does not receive the v3 initialization structure; Tier 1 relay remains the normal fallback when production APO binding is unavailable.

## First-party virtual driver

The Voxveil-owned render-only driver is derived from the pinned Microsoft SysVAD sample boundary. It exposes one render endpoint and no microphone/capture, sideband, hardware-offload, or sample APO surface. Its advertised mix path is stereo 48 kHz float32 so it matches the user-mode relay contract.

The Tier 2 driver INF intentionally follows the pinned SysVAD model applicability floor: x64 and Arm64 installation is restricted to Windows build 22621 (Windows 11 22H2) or later. This is a driver-component boundary, not an overall Voxveil Windows support floor; supported Windows 10 systems continue to use Tier 1 relay/fallback paths rather than installing this first-party driver.

Unsigned or test-signed driver output is development-only and must never be staged into a retail release. The local WDK project has signing disabled; repository scripts prepare deterministic x64/Arm64 packages and attestation CABs, then verify returned Microsoft-signed packages before staging.

Microsoft attestation signing is treated as a pilot/direct-validation path rather than Windows Certified retail qualification. Retail staging requires WHCP/HLK or another explicitly Microsoft-approved production signing/distribution path applicable to the exact package.

Driver submission, EV-certificate operations, Hardware Dev Center/Partner Center review, and WHCP/HLK qualification are external release operations. Repository tooling may prepare and validate packages but does not store signing secrets.

## Safety

- no arbitrary network service or generic URL launcher;
- no silent third-party driver download or install;
- signed production system components only for release channels that require them;
- never select the interception endpoint as the physical sink;
- persist endpoint IDs rather than display names;
- communication sessions may remain bypassed according to policy;
- no UI, disk, process launch, network, COM activation, property-store I/O, logging, allocation, or locks on the real-time APO/relay processing path;
- bounded buffering only;
- on unrecoverable relay failure, report `faulted` and stop claiming active processing;
- stale/non-default APO instances must not keep runtime priority or remain enabled while a virtual relay is the active Voxveil processing path;
- APO uninstall must be component-scoped and must not remove the first-party virtual driver;
- never persist captured/render audio from the virtual-driver sample path.

## Verification

Automated verification covers backend precedence, endpoint-scoped APO coverage, virtual-endpoint classification, readiness transitions, feedback prevention, endpoint-ID persistence, relay lifecycle, sample processing, Tauri DTO mapping, UI states, signed APO staging/integrity markers, component-scoped uninstall, driver/package invariants, and CAPX policy helpers.

Real release verification additionally requires Windows hardware testing for:

- supported virtual endpoint absent/present/default routing;
- USB, HDMI/DisplayPort, Bluetooth, and onboard endpoints where supported;
- device removal and stale endpoint IDs;
- default-output transitions between an APO-installed physical endpoint and VB-CABLE/`Voxveil Input` while old AudioDG graphs are still alive;
- different shared-mode sample rates;
- clean enable/disable and process exit;
- proof that no duplicate unprocessed stream reaches the physical sink;
- loaded-APO precedence only on the endpoint recorded by the installer;
- APO uninstall while the first-party virtual driver remains installed;
- CAPX discovery-only versus real-processing instance accounting;
- effect-state/app-state independence;
- supported Windows 10/11 release builds according to the product support policy, with first-party Tier 2 driver install tests beginning at build 22621;
- Secure Boot and `TESTSIGNING=Off` for production-driver validation;
- applicable HLK/WHCP qualification for retail claims.

See:

- `docs/testing/windows-vb-cable-relay.md`
- `docs/testing/windows-apo-capx-hlk.md`
- `docs/release/windows-apo-production-gate.md`
