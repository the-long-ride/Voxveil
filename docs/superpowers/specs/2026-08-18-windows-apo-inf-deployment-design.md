# Windows APO INF Deployment Design

## Goal

Replace Voxveil's direct writes to protected MMDevices endpoint `FxProperties` with the Windows componentized APO driver-package model while preserving the single-download `voxveil.exe` UX.

## Root cause

The development installer currently elevates successfully but receives `SecurityException: Requested registry access is not allowed` when it attempts to create an endpoint `FxProperties` key under `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render`. This registry location is owned by the Windows audio stack and is not a supported general-purpose installation surface.

## Supported deployment model

Voxveil follows Microsoft's componentized SysVAD pattern:

1. An audio **Extension INF** targets the existing render audio function device.
2. The extension uses `AddComponent` to create the Voxveil APO software component (`VEN_VOXV&CID_APO`).
3. The extension adds endpoint-effect properties to the existing audio topology interface via an INF `AddInterface` section.
4. A dedicated APO INF installs `VoxveilApo.dll`, registers its CLSID under the component-relative `HKR\Classes\CLSID`, and registers it with `HKR\AudioEngine\AudioProcessingObjects`.
5. Windows Audio Endpoint Builder materializes the endpoint effect property store and builds the graph. Voxveil never writes `MMDevices\...\FxProperties` directly.

For Windows 11 21H2 and later, the APO INF uses `Class=AudioProcessingObject` and class GUID `{5989fce8-9cd0-467d-8a6a-5419e31529d4}`. The component hardware ID is `SWC\VEN_VOXV&CID_APO`, matching the current Microsoft componentized APO sample pattern.

## Endpoint targeting

The elevated installer discovers enabled render interfaces with `pnputil /enum-interfaces /class {65E8773E-8F56-11D0-A3B9-00A0C9223196} /enabled /instanceid` and obtains each render device's hardware IDs with `pnputil /enum-devices /instanceid <id> /ids`.

For each render device, it enumerates enabled topology interfaces (`KSCATEGORY_TOPOLOGY`) and associates interfaces with the same device instance ID. The exact reference string is recovered from the topology interface path when present; if no explicit reference string is present, the INF emits an empty reference string. This avoids assuming every OEM uses `Topology`, while still supporting the standard WDK convention.

The generated extension INF contains one model/install section per unique render hardware ID and one `AddInterface` line per matching topology interface.

## Endpoint-effect properties

The topology interface AddReg section writes only through `HKR,FX\0` inside the Extension INF:

- `PKEY_FX_Association = {D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},0`
- `PKEY_CompositeFX_EndpointEffectClsid = {D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},15`
- `PKEY_EFX_ProcessingModes_Supported_For_Streaming = {D3993A3F-99C2-4402-B5EC-A92A0367664B},7`
- processing mode `AUDIO_SIGNALPROCESSINGMODE_DEFAULT = {C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}`

The composite endpoint property is used so Voxveil can coexist with another endpoint APO position instead of intentionally replacing the single-value property.

## Development signing policy

This iteration is a development build. Runtime-generated target-specific Extension INFs cannot have a pre-generated catalog because the hardware IDs and interface reference strings are machine-specific. The installer therefore uses normal elevated PnP package installation and preserves all `pnputil`/SetupAPI diagnostics if Windows refuses an unsigned package.

The design does **not** enable `TESTSIGNING`, alter BCD, disable Secure Boot, take ownership of protected registry keys, or run registry edits as SYSTEM. `DisableProtectedAudioDG=1` remains a clearly marked development-only setting while the APO DLL is unsigned; uninstall restores its previous value.

A production release requires stable target packages and release-signed catalogs/driver packages.

## Installation flow

`voxveil.exe` embeds:

- `VoxveilApo.dll`
- `VoxveilApoCheck.exe`
- `VoxveilApo.inf`
- Extension INF template/generator logic
- installer/uninstaller scripts

On Install:

1. Extract embedded payload to a temporary directory.
2. Elevate once.
3. Validate native COM activation with `VoxveilApoCheck.exe`.
4. Discover enabled render devices and topology interfaces.
5. Generate `VoxveilAudioExtension.inf` for those actual devices.
6. Stage/install the extension package and APO package using `pnputil /add-driver ... /install`.
7. Store installed package/target metadata in `%ProgramData%\Voxveil\apo-installed.json`.
8. Keep the control file at `%ProgramData%\Voxveil\apo-control.bin`.
9. Restart audio services when possible and require a reboot when PnP reports one is necessary.
10. Reprobe from Voxveil.

## Failure diagnostics

Any PnP failure returns:

- process exit code
- complete `pnputil` stdout/stderr
- generated INF path/content summary
- discovered render device instance IDs/hardware IDs/topology refs
- relevant tail of `%WINDIR%\INF\setupapi.dev.log`

The existing UI diagnostics modal automatically opens on failure and provides Copy details.

## Uninstall

Uninstall removes Voxveil's OEM extension/APO packages through `pnputil /delete-driver <oemN.inf> /uninstall /force` using package names captured at install time, removes Voxveil state/control files, restores `DisableProtectedAudioDG`, and restarts audio services. It does not restore endpoint registry backups because the new installer never directly modifies those keys.

## Safety constraints

- No direct writes to `MMDevices\Audio\Render\*\FxProperties`.
- No ACL changes/takeown on Windows audio registry keys.
- No `bcdedit` or Windows Test Mode.
- No replacement of the OEM base audio driver.
- Preserve other endpoint APOs by using the composite EFX property.
- Fail closed when no render target/topology interface can be resolved.
- Keep permanent `.github/workflows` empty/absent; verification workflows are disposable only.
