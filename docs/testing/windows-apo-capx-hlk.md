# Windows APO CAPX and HLK Validation

This matrix is the release-time verification contract for the Voxveil Windows APO. A successful local build, test-signed install, or Microsoft attestation signature is not by itself a production qualification result.

## Preconditions

- Use a clean supported Windows image with Secure Boot enabled unless a specific negative test requires otherwise.
- `TESTSIGNING` must be `Off` for production-validation runs.
- Install only the package set intended for the release channel under test.
- Record Windows build, machine model, audio adapter/driver version, package hashes, catalog signer, APO package version, and test timestamp.
- For Windows 11 CAPX runs, the endpoint extension INF must contain the fixed Voxveil property context `{63E268CE-4CBC-48E0-BEB6-55103316F477}` and must not use the legacy root `PKEY_FX_Association` for that endpoint.
- Production installability must come from a package staged through the signed-APO verifier. `apo-verification.json` must be present and the elevated installer must re-hash all five signed artifacts before modifying PnP/audio state.

## Automated preflight

Run from an x64 Developer PowerShell:

```powershell
npm run test:quality
cargo test -p voxveil-windows-audio
npm run build:windows -- -SkipNpmInstall
```

The Windows build must compile and run `VoxveilApoPolicyTests.exe` before building the production APO.

## Windows 10 compatibility matrix

Windows 10 does not pass `APOInitSystemEffects3`. Validate the older initialization path instead:

| Check | Expected |
| --- | --- |
| APO initializes through v1/v2 | success |
| `voxveil-control status` | `loaded>=1`, `capx=0` when CAPX export is present |
| App master disabled | no Voxveil DSP |
| App master enabled, vocal=100 | passthrough |
| App master enabled, vocal<100 | center suppression active |
| Device removal / graph rebuild | no stale `ready` state |
| Process exit | no persistent audio capture or output worker |

Windows 10 compatibility in this workstream is source/init-path compatibility. Tier 1 relay remains the normal fallback when an APO cannot be safely production-bound to a Windows 10 endpoint.

## Windows 11 CAPX matrix

Use `scripts/windows/probe-apo-capx.ps1` for non-mutating status checks.

| Phase | Expected |
| --- | --- |
| Effects discovery with no playback graph | `capx` may be `>=1`; `loaded=0` |
| Discovery-only initialization | never increments `loaded` |
| Real playback graph on the endpoint recorded in `install-state.json` | `loaded>=1`; APO may become the runtime backend only while that endpoint is also the current default render endpoint |
| Controllable effect query | exactly the Voxveil vocal-suppression effect is exposed outside RAW mode |
| Effect state OFF | app master cannot force DSP on |
| Effect state ON + app master OFF | Windows state cannot force DSP on |
| Effect state ON + app master ON + vocal<100 | DSP active |
| RAW processing mode | controllable/legacy effect lists empty; no Voxveil DSP |
| Graph teardown | `loaded` returns to zero after real instances are released |
| Discovery teardown | `capx` returns as discovery instances are released |

The fixed effect GUID is `{B9FD554E-8F72-4B20-9AB1-13F8E8BFDD02}`.

## Default-endpoint handoff matrix

APO readiness is endpoint-scoped. A process-wide `loaded>0` count is not sufficient when Windows has moved ordinary playback to another endpoint.

| Transition | Expected |
| --- | --- |
| APO-installed physical endpoint is default and has a real loaded instance | APO has priority and may report `ready` |
| Change Windows default to standard VB-CABLE while the old physical APO graph still exists | old APO must not keep runtime priority; Voxveil app gate for that APO is disabled before relay processing; VB-CABLE relay remains eligible |
| Change Windows default to `Voxveil Input` while the old physical APO graph still exists | old APO must not keep runtime priority; first-party virtual relay remains eligible; no double Voxveil DSP |
| Change back to the APO-installed physical endpoint | APO may regain priority only after a real non-discovery instance is loaded on that endpoint |
| `install-state.json` is missing for an otherwise loaded APO | coverage is unproven; do not report APO `ready` |
| `install-state.json` names another endpoint | coverage is unproven for the current default; do not report APO `ready` |
| malformed endpoint-scoped install state | fail closed and surface a backend fault rather than guessing |

For each transition, verify there is exactly one Voxveil processing path and no relay/APO double-processing or duplicate unprocessed physical stream.

## Install/uninstall ownership matrix

The componentized APO installer and Tier 2 virtual driver can coexist. Their package ownership must remain independent. One install-state file manages exactly one APO endpoint at a time; changing the managed endpoint requires scoped uninstall first.

| Check | Expected |
| --- | --- |
| Production APO package staged through verifier | `apo-verification.json` exists and hashes match the five signed APO/Extension artifacts immediately before installation |
| Legacy runtime-interface development install | persist whether `attach-effects` actually succeeded; uninstall calls `detach-effects` only for recorded attachment ownership, checkpoints `legacyRuntimeAttached=false`, then deletes APO packages |
| TestSign failure before runtime attachment | uninstall skips `detach-effects` and continues scoped package/certificate cleanup instead of treating binding mode alone as proof of attachment |
| APO install while `VoxveilVirtualAudio` is already installed | `install-state.json.installedInfNames` contains only the APO/Extension packages added or previously owned by the APO installer |
| Different endpoint requested while endpoint-scoped APO install state exists | reject before any PnP mutation, even when the previous endpoint state is partial/non-ready; uninstall the managed APO state/packages first |
| Multiple installable playback endpoints are present | expose explicit per-endpoint install actions only; do not bulk-install because `loaded>=1` is process-wide and cannot prove which newly targeted endpoint loaded |
| APO install returns PnPUtil `3010` | `install-state.json` remains scoped to the exact owned APO/Extension packages with `bindingReady=false`; record the restart-required result, restart Windows, rerun the same endpoint installation, and do not accept APO `ready` until a later `loaded>=1` verification succeeds |
| APO uninstall with first-party virtual driver installed | virtual driver remains installed and `Voxveil Input` remains available |
| APO uninstall returns PnPUtil `3010` | the successfully deleted INF is removed from `installedInfNames` but retained separately as `pendingRemovedInfName`; the remaining `installedInfNames` are persisted in `install-state.json`; after restart, rerun uninstall and prove the pending removed INF is absent from Driver Store before clearing that identity or deleting any remaining recorded package |
| Missing/old install state with no recorded APO INF names | uninstaller removes no provider-wide driver packages; manual cleanup is required instead of guessing |
| TestSign development install/retry | exact generated certificate thumbprint is persisted and reused; no additional development certificate is created while owned state remains |
| TestSign development uninstall | recorded certificate is removed from `LocalMachine\\My`, `Root`, and `TrustedPublisher` only after owned APO packages are gone |
| Final package removed but `AudioSrv` restart fails | keep `install-state.json` with `audioServiceRestartRequired=true`; rerun uninstall retries the restart before certificate/state cleanup, and new install is blocked meanwhile |
| Successful APO uninstall | recorded install state is removed only after AudioSrv rebuilds without stale Voxveil APO registration and any scoped TestSign certificate cleanup succeeds |

For every `3010` case, record which package operation requested the restart, the exact `installedInfNames` state before reboot, any `pendingRemovedInfName`, the post-restart rerun result, and the final AudioDG/readiness or uninstall outcome. After restart, record the Driver Store absence proof for the pending removed INF before its identity is cleared. A `3010` result is successful completion requiring restart, not a hard package failure.

## Endpoint and hardware matrix

Run supported combinations that apply to the release claim:

- onboard analog speakers/headphones;
- USB audio;
- HDMI/DisplayPort audio;
- Bluetooth output where APO binding is supported by the target driver model;
- endpoint default-device changes;
- endpoint disable/enable;
- unplug/replug;
- sleep/resume;
- reboot;
- user sign-out/sign-in;
- multiple render endpoints present simultaneously.

For each case verify AudioDG stability, correct endpoint binding, no duplicate unprocessed path introduced by Voxveil, and no readiness false positive.

## HLK / WHCP gate

For a retail production claim:

1. Select the applicable Windows Hardware Lab Kit playlist for the target Windows release and audio device/driver category.
2. Run all required audio/device tests and any Microsoft-required supplemental tests or filters.
3. Preserve the HLK project/package and submission evidence outside the source repository according to release records policy.
4. Resolve failures rather than suppressing them unless Microsoft explicitly documents an applicable erratum/filter.
5. Submit through the approved Hardware Dev Center/Partner Center path when WHCP signing/certification is required.
6. Verify the returned Microsoft-signed package again before release staging.

The repository does not contain EV private keys, Partner Center credentials, HLK result bundles, or downloaded signed release artifacts.

## Stop conditions

Do not mark the APO production-qualified if any of these are true:

- `TESTSIGNING` is enabled for the validation result being cited;
- discovery-only initialization causes `loaded>0`;
- a loaded APO on a non-default/stale endpoint causes the backend to report APO `ready`;
- changing the default endpoint can leave both the APO app gate and a virtual relay processing the same Voxveil route;
- production installation invokes `voxveil-control attach-effects`;
- the production signed artifacts do not match `apo-verification.json` at install time;
- APO uninstall can remove an unrelated Voxveil virtual-driver package;
- the extension INF mixes context CAPX association with legacy root association for the same endpoint;
- AudioDG crashes, hangs, or repeatedly rebuilds the graph;
- the effect list/state does not match actual DSP behavior;
- a PnPUtil `3010` result is treated as a hard failure, or restart/resume evidence is missing for the operation that produced it;
- the package lacks the required Microsoft/WHCP release evidence for the intended channel.
