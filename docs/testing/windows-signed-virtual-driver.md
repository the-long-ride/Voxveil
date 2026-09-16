# Windows signed virtual-driver validation

Use this procedure only with the exact Microsoft-signed package intended for the selected Voxveil release channel. Do not substitute an unsigned or locally test-signed package.

The first-party Tier 2 driver INF follows the pinned SysVAD applicability boundary and supports Windows build 22621 (Windows 11 22H2) or later. Windows 10 validation belongs to the Tier 1 relay/fallback path rather than this driver-install matrix.

## Package identity

- Voxveil commit: ______________________________________
- Release channel: ☐ Pilot ☐ Retail
- Signing path/evidence: _______________________________
- Architecture: ☐ x64 ☐ Arm64
- `VoxveilVirtualAudio.inf` SHA-256: ___________________
- `VoxveilVirtualAudio.cat` SHA-256: ___________________
- `VoxveilVirtualAudio.sys` SHA-256: ___________________
- Catalog signer summary: ______________________________
- Microsoft signature verification summary: ___________

## Machine identity

- Windows edition/build (must be 22621+): ______________
- Architecture: ________________________________________
- Secure Boot state: ___________________________________
- TESTSIGNING state: ___________________________________
- Validation date/operator: ____________________________

## Pre-install checks

Run from an elevated PowerShell:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/verify-signed-virtual-driver.ps1 `
  -PackageDir <package> `
  -Architecture x64

Confirm-SecureBootUEFI
bcdedit /enum | Select-String testsigning
```

- [ ] Machine build is Windows 11 build 22621 or later.
- [ ] Selected package architecture matches the machine's native Windows architecture (x64 or Arm64).
- [ ] Signed-package verification succeeds.
- [ ] Secure Boot is enabled.
- [ ] TESTSIGNING is not enabled.
- [ ] No local development/test certificate was imported for this package.
- [ ] For a retail run, `release-evidence.json` matches the verified package hashes and approved signing path.

## Release-package staging

Build the desktop package with the exact verified driver input:

```powershell
$env:VOXVEIL_SIGNED_DRIVER_DIR = '<verified-returned-package>'
$env:VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL = 'Retail'
npm run build:windows
```

Verify `dist/windows-x64/Voxveil/system-audio/virtual-driver/` contains only the staged INF/CAT/SYS plus `verification.json`, and that the package also contains:

```text
system-audio/voxveil-virtual-device.exe
system-audio/install-staged-virtual-driver.ps1
system-audio/uninstall-staged-virtual-driver.ps1
```

`voxveil-virtual-device.exe` is the repository-owned SetupAPI helper used only to create/reuse and remove the exact `Root\VoxveilVirtualAudio` devnode. The lifecycle scripts keep driver-store installation/deletion in PnPUtil. They must not enable TESTSIGNING, import a local certificate, redistribute DevCon, or search/delete every Voxveil driver package.

## Clean installation

From an elevated PowerShell in the staged `system-audio` directory:

```powershell
.\install-staged-virtual-driver.ps1
```

The installer re-hashes the INF/CAT/SYS against `virtual-driver/verification.json`, verifies the fixed Voxveil driver/interface identity and Microsoft catalog signer, resolves native processor architecture through `Win32_Processor`, and rejects a package/OS architecture mismatch before creating or reusing the root devnode. Only then does it ensure exactly one `Root\VoxveilVirtualAudio` devnode through `voxveil-virtual-device.exe` and invoke `pnputil /add-driver ... /install`. It resolves the installed driver from the exact returned device instance and records both that instance ID and one published INF in `virtual-driver/virtual-driver-install-state.json`.

- [ ] Wrong-architecture staged packages are rejected before any root-devnode mutation.
- [ ] The helper reports exactly one Voxveil root devnode.
- [ ] PnPUtil succeeds without enabling test mode.
- [ ] `virtual-driver-install-state.json` contains one `deviceInstanceId` and one `oemN.inf` `publishedInf`.
- [ ] The recorded `deviceInstanceId` is the `Voxveil Virtual Audio` devnode bound to the recorded `publishedInf`.
- [ ] Device Manager reports `Voxveil Virtual Audio` without signature/code errors.
- [ ] Windows Sound exposes exactly one Voxveil render endpoint named `Voxveil Input`.
- [ ] No Voxveil microphone/capture endpoint is created.
- [ ] No sample SysVAD/Contoso/Tablet Audio device identity appears.

Do not treat this command by itself as a clean-install test:

```powershell
pnputil /add-driver .\virtual-driver\VoxveilVirtualAudio.inf /install
```

PnPUtil installs or updates matching existing devices; it does not create Voxveil's root-enumerated devnode. Test the shipped lifecycle script for the release path.

## Voxveil relay behavior

With no endpoint-scoped loaded Voxveil APO:

1. Make `Voxveil Input` the ordinary Windows default render endpoint.
2. Select a distinct physical output in Voxveil.
3. Enable Voxveil processing.

Verify:

- [ ] backend kind becomes `voxveil-cable-relay`;
- [ ] backend state reaches `ready` only after capture + DSP + physical render start;
- [ ] the same Tier 1 relay/DSP behavior works without a separate driver-specific data path;
- [ ] vocal `100` behaves as bypass for the supported path;
- [ ] vocal `0` audibly changes centered stereo test content;
- [ ] no unprocessed duplicate stream reaches the physical endpoint;
- [ ] selecting the Voxveil virtual endpoint as the physical sink is rejected.

With both standard VB-CABLE and Voxveil Virtual Audio installed:

- [ ] an endpoint-scoped loaded APO on the current default physical endpoint has highest precedence;
- [ ] a stale loaded APO on another endpoint does not suppress the virtual relay;
- [ ] when APO does not cover the current default route, the signed Voxveil virtual endpoint is preferred over VB-CABLE when it is the controlled/default route;
- [ ] VB-CABLE remains usable as fallback when the Voxveil driver is absent/unavailable.

## Uninstall and reinstall

From the staged `system-audio` directory:

```powershell
.\uninstall-staged-virtual-driver.ps1
```

The uninstaller revalidates the recorded `publishedInf` against the recorded `deviceInstanceId`, removes only that exact `Root\VoxveilVirtualAudio` devnode through the SetupAPI helper, then deletes only the recorded driver-store package. If an interrupted prior uninstall already removed the devnode, it verifies the current `%WINDIR%\INF\oemN.inf` still carries Voxveil's exact hardware/provider identity before package deletion.

- [ ] Uninstall removes only the `deviceInstanceId` and `publishedInf` recorded in `virtual-driver-install-state.json`.
- [ ] The script refuses broad deletion when the recorded INF is bound to multiple devices or a non-Voxveil device.
- [ ] The script does not enumerate/delete every driver whose provider is Voxveil.
- [ ] If install state is missing, the script removes no device/package rather than guessing.
- [ ] Uninstall succeeds; reboot if Windows requests it.
- [ ] `Voxveil Input` is gone after uninstall/reboot.
- [ ] No stale Voxveil virtual endpoint remains.
- [ ] Any independently installed Voxveil APO/Extension packages remain installed.
- [ ] Reinstalling the exact same signed package through `install-staged-virtual-driver.ps1` succeeds without creating a duplicate Voxveil devnode.
- [ ] Voxveil handles the endpoint disappearance without feedback recursion or false `ready` state.

For support diagnostics only, the equivalent operations are scoped to the exact recorded identities:

```powershell
pnputil /remove-device "<deviceInstanceId>"
pnputil /delete-driver <oemN.inf>
```

Do not replace the shipped lifecycle scripts with provider-wide or wildcard cleanup.

## Repository verification

On the configured Windows development machine run:

```powershell
npm run quality
cargo test --workspace
npm run typecheck
npm run build:windows
```

- [ ] All required repository checks pass.
- [ ] `npm run build:windows` builds and stages `system-audio/voxveil-virtual-device.exe`.
- [ ] `npm run build:windows` includes `system-audio/virtual-driver` only when `VOXVEIL_SIGNED_DRIVER_DIR` is explicitly provided and verification succeeds.
- [ ] The staged desktop package contains the scoped virtual-driver helper and install/uninstall scripts.
- [ ] Pilot packages require `VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL=Pilot`.
- [ ] Retail staging refuses packages without matching retail evidence.

## Result

Result: ☐ Pass ☐ Fail ☐ Blocked

Blocking defects / notes:

________________________________________________________________________

________________________________________________________________________
