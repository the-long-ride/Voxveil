# Windows APO Production Release Gate

This document defines when Voxveil may describe the Windows APO/CAPX path as production-ready. Source completion, local execution, test signing, and attestation signing are separate milestones and must not be conflated.

## Release channels

### Development

Allowed:

- local unsigned builds;
- test-signed APO/extension packages;
- `TESTSIGNING` on dedicated development machines;
- explicit legacy `voxveil-control attach-effects` runtime registry mutation.

Not allowed:

- distribution as a normal end-user production package;
- claims of Microsoft certification or Secure-Boot production readiness.

### Pilot

A pilot may use a Microsoft-attestation-signed virtual-driver package when the current Microsoft rules permit it and the repository verification/staging gate accepts it. Attestation is treated as trusted pilot/direct-validation signing, not Windows Certified retail qualification.

The APO package used in a pilot must still be signed through an appropriate trusted production-capable path for the environment being tested; test-signed APO packages do not become pilot production artifacts merely because the virtual driver is attestation-signed.

### Retail

Retail staging requires all of the following:

- Secure Boot compatible Microsoft-trusted driver package;
- `TESTSIGNING=Off` validation;
- WHCP/HLK or another explicitly Microsoft-approved production distribution/signing path applicable to the package;
- matching package hashes and release evidence;
- production CAPX endpoint extension binding on Windows 11;
- no runtime `attach-effects` invocation in the production installer;
- completed supported-hardware/OS validation matrix;
- no unresolved load, stability, effects-discovery, endpoint-handoff, uninstall-ownership, or readiness defects.

## APO/CAPX package invariants

The Windows 11 production extension INF must:

- target the supported Windows 11 build range;
- keep the Voxveil ExtensionId `{1D81E93D-AB81-473B-9E5E-94FAE8D2377F}` stable across updates in this extension lineage;
- use the fixed context GUID `{63E268CE-4CBC-48E0-BEB6-55103316F477}`;
- associate `PKEY_FX_Association` under `FX\0\%VOXVEIL_APO_CONTEXT%`;
- create the CAPX `User` context store used by the APO settings framework;
- retain the stream-effect CLSID and processing-mode registration required by the audio engine;
- bind through the signed endpoint-interface section;
- contain no unresolved template markers;
- not use the legacy root `FX\0` association for the same production endpoint.

The ExtensionId is a Windows servicing/lineage identity, not a per-build nonce. The initial Partner Center submission registers it to the submitting organization; subsequent updates for the same extension lineage must reuse it and advance `DriverVer`. Do not regenerate it during packaging.

The APO itself must:

- implement `IAudioSystemEffects3` while preserving v1/v2 initialization compatibility;
- expose the fixed vocal-suppression effect GUID `{B9FD554E-8F72-4B20-9AB1-13F8E8BFDD02}`;
- treat Windows effect state and Voxveil app state as independent gates;
- never count discovery-only v3 instances as `loaded`;
- keep property-store/COM work outside `APOProcess`;
- keep `APOProcess` allocation-free, blocking-free, logging-free, registry-free, and process-launch-free.

The production-signed APO input consumed by the Windows package builder contains exactly:

```text
VoxveilApo.inf
VoxveilApo.dll
VoxveilApo.cat
VoxveilApoExtension.inf
VoxveilApoExtension.cat
```

Before a release build uses those files, `scripts/windows/verify-signed-apo-package.ps1` must verify both INF files, both Microsoft-trusted catalogs, each catalog's package membership, the fixed CLSID/CAPX/Extension identities, x64 APO architecture, and the APO DLL's Authenticode signature required by the INF `SignatureAttributes.PETrust` declaration. `scripts/windows/stage-signed-apo-package.ps1` copies only those verified files and records their hashes/signers in `apo-verification.json`.

`apo-verification.json` is part of the production installation contract, not informational metadata. Endpoint discovery must not advertise a production APO package as installable without that staging marker. Immediately before PnP installation, the elevated installer must recompute SHA-256 for all five signed artifacts and require exact matches with the verifier manifest, in addition to rechecking the fixed CAPX/Extension identities and selected endpoint binding.

For a release package, set `VOXVEIL_SIGNED_APO_DIR` to that verified returned package before running `npm run build:windows`. The build script does not copy that directory directly; it invokes the verifier/stager. When the variable is unset, the locally built APO DLL/INF remain development artifacts and the production installer intentionally lacks the catalogs/Extension INF/verifier marker needed to install them as production packages.

## Readiness rule

`loaded=<N>` remains the real-processing-instance counter, but it is not by itself sufficient to choose the APO backend.

- `loaded=0 capx>0`: CAPX discovery may be active, but the APO is **not** a real processing backend.
- `loaded>=1`: at least one successfully initialized non-discovery processing instance exists somewhere in AudioDG.
- APO runtime precedence additionally requires `install-state.json` to identify the endpoint on which Voxveil installed/bound the APO and for that endpoint to be the current default render endpoint.
- a loaded instance on another/stale endpoint does not make the current route APO-ready; Voxveil disables its app gate before virtual-relay processing so the old instance cannot double-process the relay destination.
- missing endpoint identity means APO coverage is unproven and must not produce a readiness positive. Explicit manual `legacy-reference` development state may remain unscoped, but it is not a production readiness proof.
- `capx` and `system-effect` are diagnostics/control state and cannot independently make the backend ready.

When Windows changes the default output away from the APO-installed endpoint, a stale `loaded>0` count must not suppress VB-CABLE/Voxveil virtual-endpoint fallback. When the user changes back, the APO may regain precedence only after a real non-discovery processing instance exists for the installed/default endpoint.

The current native control telemetry exposes a process-wide `loaded` count rather than endpoint-specific instance identities. Therefore one install-state file manages exactly one APO endpoint at a time. The installer must reject a different endpoint while any endpoint-scoped install state remains, including partial/non-ready state after a failed or reboot-required install. Uninstall the managed APO state/packages first, then install the different endpoint. The UI must not offer a bulk “install all endpoints” action that could imply endpoint-scoped load verification which the current telemetry cannot prove.

## Package ownership and uninstall rule

The APO/Extension installer may coexist with the first-party `VoxveilVirtualAudio` kernel package. Package ownership must remain scoped.

- `install-state.json.installedInfNames` records only APO/Extension INF names newly added by that installer invocation plus previously recorded APO/Extension names that are still installed.
- It must never be populated by enumerating every PnP driver whose provider is `Voxveil` after installation.
- `legacy-runtime-interface` uninstall first calls `detach-effects` against the exact stored topology/audio interface paths.
- The uninstaller deletes only the recorded APO/Extension INF names. If no trustworthy names are recorded, it removes no provider-wide driver packages and requires explicit/manual cleanup instead of guessing.
- Installing or uninstalling the APO must not remove the independent Tier 2 virtual driver or `Voxveil Input` endpoint.

PnPUtil exit code `3010` is successful completion requiring a Windows restart, not a hard package failure. The lifecycle remains fail-closed across that boundary:

- during base APO or Extension installation, ownership is snapshotted with `bindingReady=false` before `3010` is propagated; AudioDG restart/load validation is not run and the UI reports `reboot-required`;
- restart Windows and rerun the endpoint installation. Only a later successful `loaded>=1` verification may write `bindingReady=true`;
- during uninstall, a package that returns `3010` is removed from `installedInfNames`, the remaining `installedInfNames` are persisted to `install-state.json`, and the just-removed published INF is retained separately as `pendingRemovedInfName` before the script exits without attempting additional package deletions;
- after restart, rerun uninstall. The script must perform scoped post-reboot absence validation for `pendingRemovedInfName` in Driver Store before clearing that identity, then it may revalidate and delete any remaining recorded package. Do not delete or broaden the state manually.

Any other nonzero PnPUtil result remains a hard failure and preserves the recorded package ownership for recovery.

## Required evidence

Store release evidence outside the source repository and record at minimum:

- release version/commit;
- OS builds tested;
- hardware/audio driver matrix;
- SHA-256 package hashes;
- `apo-verification.json` and install-time re-hash result for the exact signed artifacts;
- catalog/signature verification output;
- Secure Boot and `TESTSIGNING` state;
- endpoint-default transition results proving stale APO instances do not cause false readiness/double DSP;
- component-scoped install/uninstall results, including any `3010` restart boundary and post-restart continuation, with the first-party virtual driver present;
- HLK/WHCP result identifiers where applicable;
- Partner Center submission/result identifiers where applicable;
- CAPX discovery/real-processing probe results;
- regression test/build output.

Do not store private signing keys, EV certificate material, account credentials, or secret tokens in the repository.

## Release decision

A build may be called **repository-complete** when source, tests, scripts, and documentation are present.

A build may be called **Windows-validated** only after the documented real-machine matrix passes.

A build may be called **retail production-ready** only after both Windows validation and the applicable Microsoft signing/qualification gate pass for the exact artifacts being shipped.
