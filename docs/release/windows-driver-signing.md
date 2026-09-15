# Windows virtual-driver signing and release boundary

This document covers the Voxveil-owned `Root\VoxveilVirtualAudio` kernel driver. It deliberately separates repository automation from certificate/account operations that must occur outside the repository.

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

1. pin/import the approved SysVAD source revision;
2. build the Voxveil render-only derivative;
3. generate INF/CAT/PDB submission material;
4. create the attestation CAB;
5. validate package structure and INF rules;
6. verify the returned Microsoft-signed INF/CAT/SYS package;
7. stage a verified package only when explicitly supplied to the release build.

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
3. Build a submission package locally:

   ```powershell
   npm run build:windows-driver:x64
   # and, when shipping Arm64:
   npm run build:windows-driver:arm64
   ```

4. Confirm the CAB contains a driver-package subfolder, not root-level driver files:

   ```text
   VoxveilVirtualAudio/
     VoxveilVirtualAudio.inf
     VoxveilVirtualAudio.sys
     VoxveilVirtualAudio.cat
     VoxveilVirtualAudio.pdb
   ```

5. Use the EV certificate provider's approved process outside the repository to Authenticode-sign the generated CAB with SHA-256 and a trusted timestamp.
6. Submit the EV-signed CAB through Hardware Dev Center.
7. Download the Microsoft-signed returned package.
8. Verify the returned package:

   ```powershell
   powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/verify-signed-virtual-driver.ps1 `
     -PackageDir <returned-package> `
     -Architecture x64
   ```

9. For direct pilot/testing distribution only, stage with an explicit pilot channel:

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

If `VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL` is omitted while `VOXVEIL_SIGNED_DRIVER_DIR` is set, `build-windows.ps1` deliberately defaults to `Retail`, forcing the stronger evidence gate rather than silently accepting a pilot package.

## Release-blocking validation checklist

- [ ] The exact source revision and Voxveil commit are recorded.
- [ ] Secure Boot remains enabled on the validation machine.
- [ ] TESTSIGNING is off.
- [ ] `signtool verify /kp /v` succeeds for the returned catalog/driver.
- [ ] `InfVerif` succeeds.
- [ ] Package hashes match release evidence for retail builds.
- [ ] Fresh-machine installation succeeds without importing a local test certificate.
- [ ] Device Manager reports `Voxveil Virtual Audio` without signature errors.
- [ ] `Voxveil Input` appears as the render endpoint.
- [ ] Voxveil selects `voxveil-cable-relay` when the endpoint is the active controlled route and no APO is loaded.
- [ ] Uninstall removes the package cleanly.
- [ ] Reinstall of the same signed package succeeds.
- [ ] The relevant `docs/testing/windows-signed-virtual-driver.md` matrix is completed.

No first-party driver should be described as retail production-ready until this checklist and the applicable Microsoft qualification path are complete.
