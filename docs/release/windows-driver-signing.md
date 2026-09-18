# Windows virtual-driver signing and release boundary

This document covers the Voxveil-owned `Root\VoxveilVirtualAudio` kernel driver. It deliberately separates repository automation from certificate/account operations that must occur outside the repository.

The Tier 2 driver INF follows the exact pinned SysVAD applicability boundary and targets Windows build 22621 (Windows 11 22H2) or later on x64/Arm64. This is not the overall Voxveil Windows support floor; supported Windows 10 systems use the Tier 1 relay/fallback path instead of this first-party driver.

## Microsoft status model

Current Microsoft driver-signing guidance distinguishes attestation signing from Windows Hardware Compatibility Program (WHCP/HLK) certification:

- attestation signing is a Microsoft-trusted signing path intended for testing scenarios;
- an attestation-signed driver is not Windows Certified;
- attestation-signed drivers cannot be published to Windows Update for retail audiences;
- Hardware Dev Center submissions require an account with an associated valid EV certificate;
- WHCP/HLK or another explicitly Microsoft-approved retail path is required before Voxveil marks a first-party virtual driver as retail-qualified.

Official references:

- https://learn.microsoft.com/windows-hardware/drivers/dashboard/driver-signing-offerings
- https://learn.microsoft.com/windows-hardware/drivers/dashboard/code-signing-reqs
- https://learn.microsoft.com/windows-hardware/drivers/dashboard/code-signing-attestation
- https://learn.microsoft.com/windows-hardware/drivers/dashboard/code-signing-validate

Re-check those pages before every release because Microsoft submission rules can change.

## Repository responsibilities

The repository may:

1. pin the approved Windows Driver Samples commit and exact SysVAD subtree identity;
2. materialize that verified upstream SysVAD source at build time into the ignored working tree;
3. build the Voxveil render-only derivative and validate its unsigned INF/CAT/PDB submission material;
4. create an attestation CAB explicitly at the release/submission boundary;
5. verify the returned Microsoft-signed INF/CAT/SYS package;
6. stage a verified package only when explicitly supplied to the release build.

The repository must not contain or automate access to:

- EV private keys or exportable certificate bundles;
- certificate-provider account identifiers or tokens;
- Partner Center credentials/cookies/tokens;
- private Hardware Dev Center submission metadata;
- locally generated production-signing certificates;
- TESTSIGNING enablement for release builds.

## Attestation/pilot workflow

1. Enroll the organization in Microsoft Hardware Dev Center / Partner Center.
2. Associate the required valid EV certificate with the Hardware Dev Center account.
3. Obtain the validated unsigned submission for the exact release commit.

   Preferred CI handoff: run **Manual Build** for the exact commit SHA and download the artifact named `Voxveil-windows-driver-submission-${{ github.sha }}`. The artifact is the validated `native\windows\driver\out\x64\submission` signing input produced by that exact SHA; do not reuse an artifact from an older run.

   For local reproduction, build the unsigned submission package:

   ```powershell
   npm run build:windows-driver:x64
   # and, when shipping Arm64:
   npm run build:windows-driver:arm64
   ```

   The driver build intentionally stops at the verified unsigned submission directory. For x64 that directory is `native\windows\driver\out\x64\submission`.

4. Create the attestation CAB explicitly when preparing a Hardware Dev Center submission:

   ```powershell
   $submission = 'native\windows\driver\out\x64\submission'
   $cab = 'native\windows\driver\out\x64\VoxveilVirtualAudio-attestation-x64.cab'

   powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/new-driver-attestation-cab.ps1 `
     -Architecture x64 `
     -PackageDir $submission `
     -Output $cab
   ```

   This separation keeps ordinary driver compilation/package validation independent from signing-account and submission operations.

5. Confirm the CAB contains a driver-package subfolder, not root-level driver files:

   ```text
   VoxveilVirtualAudio/
     VoxveilVirtualAudio.inf
     VoxveilVirtualAudio.sys
     VoxveilVirtualAudio.cat
     VoxveilVirtualAudio.pdb
   ```

6. Use the EV certificate provider's approved process outside the repository to Authenticode-sign the generated CAB with SHA-256 and a trusted timestamp.
7. Submit the EV-signed CAB through Hardware Dev Center.
8. Download the Microsoft-signed returned package.
9. Verify the returned package:

   ```powershell
   powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/verify-signed-virtual-driver.ps1 `
     -PackageDir <returned-package> `
     -Architecture x64
   ```

   Verification requires the returned INF to preserve the fixed Voxveil identity and build-22621+ applicability, verifies the catalog with the kernel-mode trust policy, and verifies the returned INF/SYS as members of that trusted catalog. The SYS is not required to carry a separate embedded signature when its digest is covered by the trusted Microsoft-signed catalog.

10. For direct pilot/testing distribution only, stage with an explicit pilot channel:

   ```powershell
   $env:VOXVEIL_SIGNED_DRIVER_DIR = '<returned-package>'
   $env:VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL = 'Pilot'
   npm run build:windows
   ```

Attestation signing alone does not satisfy the Voxveil retail gate.

## Retail workflow

Before `ReleaseChannel=Retail`, obtain WHCP/HLK qualification or another Microsoft-confirmed path that is explicitly suitable for the intended retail distribution.

The returned package directory must contain `release-evidence.json` with hashes matching the verified files:

```json
{
  "releaseChannel": "retail",
  "signingPath": "whcp-hlk",
  "infSha256": "<lowercase sha256>",
  "catalogSha256": "<lowercase sha256>",
  "driverSha256": "<lowercase sha256>"
}
```

Allowed `signingPath` values are:

- `whcp-hlk`
- `microsoft-approved-retail`

For a Microsoft-approved alternative, the release owner must retain the supporting Microsoft correspondence/evidence outside the public repository.

A retail build then uses:

```powershell
$env:VOXVEIL_SIGNED_DRIVER_DIR = '<verified-retail-package>'
$env:VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL = 'Retail'
npm run build:windows
```

If `VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL` is omitted while `VOXVEIL_SIGNED_DRIVER_DIR` is set, `build-windows.ps1` deliberately defaults to `Retail`, forcing the stronger evidence gate rather than silently accepting a pilot package. Retail staging copies the validated `release-evidence.json` into the staged driver directory, re-hashes that copy, and records both its SHA-256 and the validated `signingPath` in `verification.json`; the installer revalidates those fields before any devnode/PnP mutation. The same manifest also records the SHA-256 of the packaged `voxveil-virtual-device.exe` SetupAPI helper. Install and uninstall open that exact helper with a verified read lock that does not share write/delete access, hold the lock across every state-driven query/create/remove operation, and release it only when the lifecycle script finishes. Installation holds the same kind of verified lock on the staged INF/CAT/SYS from hash validation through PnPUtil, so the signed driver package cannot be replaced between verification and Driver Store mutation. Restaging preserves `virtual-driver-install-state.json` in place, refuses unexpected destination entries before cleanup, removes only known staging artifacts, and never uses an in-memory delete/restore window for lifecycle ownership. Staging is architecture-scoped to a strict descendant of `dist/windows-x64/` or `dist/windows-arm64/`, rejects junction/symlink ancestors before mutation, and the desktop Windows package builder restricts recursive cleanup to a strict descendant of `dist/windows-x64/`.

## Reboot-required lifecycle

PnPUtil exit code `3010` means the requested package operation completed successfully but Windows requires a restart. SetupAPI devnode removal can independently succeed while setting `DI_NEEDREBOOT` or `DI_NEEDRESTART`; Voxveil treats either signal as the same restart boundary rather than continuing lifecycle mutation in the same boot.

For installation, `install-staged-virtual-driver.ps1` resolves the exact published INF from the new Driver Store delta or the exact Voxveil devnode binding, writes `virtual-driver-install-state.json`, and records `pendingReboot=true` plus the current Windows boot marker before returning `3010`. A same-boot rerun is rejected before devnode or PnP mutation. Do not describe the endpoint as validated or ready at that point. Restart Windows, then rerun installation/validation so the normal binding checks can prove the exact `Voxveil Virtual Audio` package and endpoint.

Package replacement follows the same ownership boundary: uninstall the currently recorded Voxveil Virtual Audio package before installing a different signed package. A same-package repair is allowed only when the staged INF/CAT/SYS hashes exactly match the complete signed-package identity in the existing non-tombstone install state; incomplete or mismatched recorded identity is rejected before devnode or PnP mutation.

For uninstall, the script first removes the recorded devnode and reads the SetupAPI restart flags surfaced by `voxveil-virtual-device.exe`, then deletes only the recorded published INF. If either devnode removal requires restart or PnPUtil returns `3010` after successful package deletion, the script retains `virtual-driver-install-state.json` only as a reboot tombstone, sets `pendingReboot=true`, records the current boot marker in `pendingRebootBootMarker`, and sets `uninstallComplete=true`. The deleted package identity must not be acted on again. Same-boot reinstall and uninstall continuation are rejected. After Windows restarts, rerunning the uninstaller performs scoped post-reboot absence validation for the recorded Voxveil devnode and Driver Store package before removing a full ownership tombstone. A package-less rollback tombstone carries no package/devnode ownership and may be cleared once the boot marker changes. A reinstall is also allowed once the stored boot marker differs and will replace the state with the new installation record.

If SetupAPI reports restart-required devnode removal but PnPUtil then hard-fails package deletion while the package still exists, the script preserves package ownership with `uninstallComplete=false`, records the reboot marker, and requires Windows to restart before cleanup is retried. If a recorded package is already absent on entry, or a hard non-3010 delete result is followed by proof that the package is already absent, cleanup is treated as an interrupted/ambiguous prior deletion: the exact devnode is queried/removed if necessary, a completed-uninstall reboot tombstone is written, and one restart plus post-reboot absence proof is required before state is discarded. Lifecycle JSON checkpoints are written through a same-directory atomic replace so a torn state write cannot replace the last complete ownership record. Do not manually broaden cleanup to other `Voxveil` provider packages; retain the exact state/identity rules in the lifecycle scripts.

Before starting the release-blocking real-machine checklist, capture the exact returned-package and machine pre-install identity with `scripts/evaluation/collect-windows-driver-validation-evidence.ps1`. This produces ignored local evidence tied to the exact Voxveil checkout and fails closed on Windows build, architecture, Secure Boot, TESTSIGNING, Microsoft signature, and retail release-evidence requirements. It does not replace the lifecycle or HLK/WHCP checks below.

## Release-blocking validation checklist

- [ ] The exact source revision and Voxveil commit are recorded.
- [ ] The validation machine is Windows build 22621 or later for this Tier 2 driver.
- [ ] Secure Boot remains enabled on the validation machine.
- [ ] TESTSIGNING is off.
- [ ] `signtool verify /kp /v` succeeds for the returned catalog.
- [ ] `signtool verify /c <catalog> /v` confirms the returned INF and SYS are covered by that catalog.
- [ ] `InfVerif` succeeds.
- [ ] Package hashes match release evidence for retail builds.
- [ ] Fresh-machine installation succeeds without importing a local test certificate.
- [ ] If install returns `3010`, `pendingReboot=true` and the current boot marker are persisted, same-boot retry is refused, Windows is restarted, and binding validation is repeated before acceptance.
- [ ] If uninstall requires restart from either SetupAPI devnode removal or PnPUtil `3010`, the current boot marker is persisted, same-boot reinstall/cleanup is refused, Windows is restarted, and cleanup/absence validation is resumed only afterward.
- [ ] Device Manager reports `Voxveil Virtual Audio` without signature errors.
- [ ] `Voxveil Input` appears as the render endpoint.
- [ ] Voxveil selects `voxveil-cable-relay` when the endpoint is the active controlled route and no APO is loaded.
- [ ] Uninstall removes the package cleanly.
- [ ] Reinstall of the same signed package succeeds.
- [ ] The relevant `docs/testing/windows-signed-virtual-driver.md` matrix is completed.

No first-party driver should be described as retail production-ready until this checklist and the applicable Microsoft qualification path are complete.
