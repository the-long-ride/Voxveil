# Windows Tier 2 Signed Virtual Driver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Voxveil-owned, render-only SysVAD-derived virtual audio endpoint plus reproducible local package/CAB tooling so a returned Microsoft-signed package can replace VB-CABLE without TESTSIGNING.

**Architecture:** Derive a single root-enumerated WaveRT render endpoint from Microsoft SysVAD, with no microphone, Bluetooth, USB sideband, keyword detection, sample APO, or hardware-offload features. Build unsigned submission artifacts locally, submit them outside the repository through Hardware Dev Center, then accept a returned package only after catalog and Authenticode verification. At runtime, a verified Voxveil endpoint reuses the Tier 1 WASAPI loopback -> DSP -> physical-output relay and is preferred over VB-CABLE while the loaded APO remains highest priority.

**Tech Stack:** C++17, WDK/PortCls/KMDF, MSBuild, INF/CAT/CAB tooling, PowerShell, SignTool, InfVerif, Rust `voxveil-windows-audio`, existing Tier 1 relay.

**Spec:** `docs/superpowers/specs/2026-09-14-windows-signed-audio-paths-design.md`

## Global Constraints

- The driver is SysVAD-derived and must preserve Microsoft source/license notices for derived source.
- Upstream provenance is pinned to `microsoft/Windows-driver-samples@67d81f217bc01edf7a4320e4911c11065635acfa`.
- The production endpoint exposes one render device only.
- Production hardware ID is `Root\VoxveilVirtualAudio`.
- Device display name is `Voxveil Virtual Audio`; render endpoint display name is `Voxveil Input`.
- Do not ship SysVAD sample APOs, capture endpoints, Bluetooth, USB sideband, keyword detection, or offload endpoints.
- Do not commit EV private keys, certificates containing private keys, Partner Center credentials, submission cookies/tokens, or returned private portal metadata.
- Do not enable TESTSIGNING in production scripts or documentation.
- A test-signed or unsigned driver package must never be staged into an end-user Voxveil release.
- The repository remains workflow-free; signing/submission is a manual release operation.
- The loaded Voxveil APO remains highest runtime priority. A verified Voxveil virtual endpoint is preferred over VB-CABLE when the APO is not loaded.
- Keep x64 and Arm64 driver configurations in the project even if the desktop application currently releases only x64.

---

### Task 1: Record upstream provenance and licensing before importing SysVAD

**Files:**
- Create: `third_party/microsoft/windows-driver-samples/README.voxveil.md`
- Create: `third_party/microsoft/windows-driver-samples/LICENSE.txt`
- Create: `third_party/microsoft/windows-driver-samples/SOURCE_REVISION`
- Modify: `scripts/quality/check-license-metadata.mjs`
- Test: `scripts/quality/check-license-metadata.test.mjs`

**Interfaces:**
- Produces an immutable source revision contract consumed by the driver import/build documentation.

- [ ] **Step 1: Write the failing license metadata test**

Add a test requiring the pinned revision and the preserved Microsoft license:

```js
const revision = readFileSync('third_party/microsoft/windows-driver-samples/SOURCE_REVISION', 'utf8').trim();
assert.equal(revision, '67d81f217bc01edf7a4320e4911c11065635acfa');
assert.match(
  readFileSync('third_party/microsoft/windows-driver-samples/README.voxveil.md', 'utf8'),
  /microsoft\/Windows-driver-samples/,
);
assert.ok(existsSync('third_party/microsoft/windows-driver-samples/LICENSE.txt'));
```

- [ ] **Step 2: Run the quality test and confirm failure**

```powershell
node --test scripts/quality/check-license-metadata.test.mjs
```

Expected: FAIL because the third-party metadata does not exist.

- [ ] **Step 3: Add provenance files**

`SOURCE_REVISION` contains exactly:

```text
67d81f217bc01edf7a4320e4911c11065635acfa
```

`README.voxveil.md` records:

```markdown
# Microsoft Windows Driver Samples provenance

Upstream: https://github.com/microsoft/Windows-driver-samples
Revision: 67d81f217bc01edf7a4320e4911c11065635acfa
Imported area: audio/sysvad
Purpose: source basis for the Voxveil render-only virtual audio driver.

Files derived into native/windows/driver retain the applicable upstream copyright/license notices.
```

Copy the upstream root license text from that exact revision into `LICENSE.txt` unchanged.

- [ ] **Step 4: Extend the repository license check**

Make `check-license-metadata.mjs` fail when the revision file, attribution README, or license file is absent or when the revision differs from the pinned value.

- [ ] **Step 5: Run the license checks**

```powershell
node --test scripts/quality/check-license-metadata.test.mjs
npm run quality:licenses
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add third_party/microsoft/windows-driver-samples scripts/quality/check-license-metadata.mjs scripts/quality/check-license-metadata.test.mjs
git commit -m "docs(windows): record SysVAD source provenance"
```

### Task 2: Import the pinned SysVAD source snapshot reproducibly

**Files:**
- Create: `scripts/windows/import-sysvad-source.ps1`
- Create recursively: `third_party/microsoft/windows-driver-samples/audio/sysvad/**`
- Modify: `.gitignore`

**Interfaces:**
- Produces a checked-in exact `audio/sysvad` snapshot matching the pinned commit.

- [ ] **Step 1: Add a deterministic import script**

The script accepts only an empty destination or `-Force`, downloads an archive of the pinned commit, copies only `audio/sysvad`, and verifies `SOURCE_REVISION` before writing:

```powershell
param([switch]$Force)
$ErrorActionPreference = 'Stop'
$Revision = '67d81f217bc01edf7a4320e4911c11065635acfa'
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Destination = Join-Path $RepoRoot 'third_party\microsoft\windows-driver-samples\audio\sysvad'
$Archive = Join-Path $env:TEMP "windows-driver-samples-$Revision.zip"
$Extract = Join-Path $env:TEMP "windows-driver-samples-$Revision"
$url = "https://github.com/microsoft/Windows-driver-samples/archive/$Revision.zip"
```

The script must:

1. reject a non-empty destination unless `-Force` is supplied;
2. download with `Invoke-WebRequest`;
3. expand to a temporary directory;
4. copy only `<archive-root>/audio/sysvad`;
5. remove `Debug`, `Release`, `.vs`, build products, `.cer`, `.cat`, `.sys`, `.dll`, `.pdb` if present;
6. leave provenance files outside the imported subtree untouched;
7. remove its temporary archive/extract directory in `finally`.

- [ ] **Step 2: Add generated/build exclusions**

Add only build outputs to `.gitignore`:

```gitignore
native/windows/driver/out/
native/windows/driver/submission/
native/windows/driver/signed/
*.cab
```

Do not ignore `.inf`, `.inx`, `.vcxproj`, `.cpp`, or `.h` source files.

- [ ] **Step 3: Execute the import once**

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/import-sysvad-source.ps1
```

Expected: `third_party/microsoft/windows-driver-samples/audio/sysvad/README.md`, `EndpointsCommon`, and `TabletAudioSample` exist at the pinned snapshot.

- [ ] **Step 4: Verify no build/signing artifacts were imported**

```powershell
$bad = Get-ChildItem third_party/microsoft/windows-driver-samples/audio/sysvad -Recurse -File |
  Where-Object Extension -in '.sys','.dll','.pdb','.cat','.cer'
if ($bad) { throw "Imported binary/signing artifacts: $($bad.FullName -join ', ')" }
```

Expected: no output and exit 0.

- [ ] **Step 5: Commit**

```bash
git add .gitignore scripts/windows/import-sysvad-source.ps1 third_party/microsoft/windows-driver-samples/audio/sysvad
git commit -m "build(windows): vendor pinned SysVAD source"
```

### Task 3: Create a render-only Voxveil SysVAD derivative

**Files:**
- Create: `native/windows/driver/VoxveilVirtualAudio.vcxproj`
- Create: `native/windows/driver/VoxveilVirtualAudio.vcxproj.filters`
- Create: `native/windows/driver/VoxveilVirtualAudio.rc`
- Create: `native/windows/driver/voxveil_endpoint.h`
- Create: `native/windows/driver/voxveil_endpoint.cpp`
- Create: `native/windows/driver/voxveil_wavert.h`
- Create: `native/windows/driver/voxveil_wavert.cpp`
- Create: `native/windows/driver/voxveil_ids.h`
- Create: `native/windows/driver/NOTICE.md`
- Reuse read-only source from: `third_party/microsoft/windows-driver-samples/audio/sysvad/adapter.cpp`, `basetopo.cpp`, `common.cpp`, `hw.cpp`, `kshelper.cpp`, `savedata.cpp`, `EndpointsCommon/minwavert.cpp`, `EndpointsCommon/minwavertstream.cpp`, `EndpointsCommon/NewDelete.cpp` and required headers from the pinned tree.

**Interfaces:**
- Produces `VoxveilVirtualAudio.sys` exposing one WaveRT render endpoint.
- Hardware identity is independent of APO component identity.

- [ ] **Step 1: Copy the minimum speaker endpoint behavior into Voxveil-owned derived files**

Use the pinned SysVAD speaker/topology and WaveRT tables as the source basis, but define only one endpoint descriptor in `voxveil_endpoint.cpp`:

```cpp
const ENDPOINT_MINIPAIR g_VoxveilRenderEndpoints[] = {
    {
        eVoxveilSpeakerDevice,
        L"Voxveil Input",
        // one host render WaveRT miniport + one topology miniport
    },
};
```

Remove capture mini-pairs and all endpoint descriptors for HDMI, SPDIF, Bluetooth, USB, microphones, and offload. Preserve upstream copyright headers on copied/derived implementations.

- [ ] **Step 2: Define Voxveil-owned stable identifiers**

`voxveil_ids.h` must define new GUIDs generated once and then treated as stable:

```cpp
// The executor generates these once with New-Guid and commits the literal values.
DEFINE_GUID(GUID_DEVINTERFACE_VOXVEIL_VIRTUAL_AUDIO, ...);
DEFINE_GUID(KSNODETYPE_VOXVEIL_VIRTUAL_SPEAKER, ...);
```

Do not reuse Microsoft sample GUIDs for Voxveil-specific device/interface identity. The generated literal GUIDs must be used consistently by source and INF before this task is committed.

- [ ] **Step 3: Build a project with only x64/Arm64 and the render-only sources**

The project must use:

```xml
<PlatformToolset>WindowsKernelModeDriver10.0</PlatformToolset>
<DriverTargetPlatform>Universal</DriverTargetPlatform>
<ConfigurationType>Driver</ConfigurationType>
<LanguageStandard>stdcpp17</LanguageStandard>
<TreatWarningAsError>true</TreatWarningAsError>
```

Configurations are `Debug|x64`, `Release|x64`, `Debug|ARM64`, `Release|ARM64`. Link `portcls.lib`, `stdunk.lib`, and `libcntpr.lib`. Do not compile the pinned sample's A2DP, BTH HFP, USB sideband, microphone, HDMI, SPDIF, tone generator, or sample APO sources.

- [ ] **Step 4: Add compile-time endpoint-count guards**

```cpp
static_assert(ARRAYSIZE(g_VoxveilRenderEndpoints) == 1, "Voxveil production driver exposes exactly one render endpoint");
static_assert(ARRAYSIZE(g_VoxveilCaptureEndpoints) == 0, "Voxveil production driver must not expose capture endpoints");
```

Represent the capture set as a zero-count pointer/count pair if the compiler rejects a zero-length C++ array.

- [ ] **Step 5: Build both release architectures**

```powershell
msbuild native/windows/driver/VoxveilVirtualAudio.vcxproj /m /p:Configuration=Release /p:Platform=x64
msbuild native/windows/driver/VoxveilVirtualAudio.vcxproj /m /p:Configuration=Release /p:Platform=ARM64
```

Expected: `VoxveilVirtualAudio.sys` for both architectures and no sample APO DLLs.

- [ ] **Step 6: Commit**

```bash
git add native/windows/driver
git commit -m "feat(windows): add render-only Voxveil virtual audio driver"
```

### Task 4: Add production INF identities and package validation

**Files:**
- Create: `native/windows/driver/package/VoxveilVirtualAudio.inf`
- Create: `scripts/windows/validate-virtual-driver-package.ps1`
- Create: `scripts/quality/check-windows-driver-package.test.mjs`

**Interfaces:**
- Produces a root-enumerated package for `Root\VoxveilVirtualAudio`.
- Validation refuses sample identities and production-forbidden files.

- [ ] **Step 1: Write the package invariant test first**

```js
const inf = readFileSync('native/windows/driver/package/VoxveilVirtualAudio.inf', 'utf8');
assert.match(inf, /Root\\VoxveilVirtualAudio/i);
assert.match(inf, /Voxveil Virtual Audio/i);
assert.doesNotMatch(inf, /Sysvad_|Tablet Audio Sample|Contoso|SwapAPO|DelayAPO/i);
```

- [ ] **Step 2: Run and confirm failure**

```powershell
node --test scripts/quality/check-windows-driver-package.test.mjs
```

Expected: FAIL because the INF does not exist.

- [ ] **Step 3: Create the componentized base driver INF**

Required identity and package lines:

```ini
[Version]
Signature="$WINDOWS NT$"
Class=MEDIA
ClassGuid={4d36e96c-e325-11ce-bfc1-08002be10318}
Provider=%ProviderName%
CatalogFile=VoxveilVirtualAudio.cat
PnpLockDown=1

[Manufacturer]
%MfgName%=Voxveil,NTamd64,NTarm64

[Voxveil.NTamd64]
%DeviceName%=Voxveil_Install,Root\VoxveilVirtualAudio

[Voxveil.NTarm64]
%DeviceName%=Voxveil_Install,Root\VoxveilVirtualAudio
```

Use architecture-correct copy sections for `VoxveilVirtualAudio.sys`, register only the render endpoint properties required by the derived driver, and set strings:

```ini
ProviderName="Voxveil"
MfgName="Voxveil"
DeviceName="Voxveil Virtual Audio"
EndpointName="Voxveil Input"
```

Do not include any test certificate or co-installer.

- [ ] **Step 4: Add PowerShell validation**

`validate-virtual-driver-package.ps1 -PackageDir <path>` must:

1. require exactly one `.inf`, one `.cat`, and one `.sys` per architecture package;
2. reject `.cer`, `.pfx`, `.pvk`, `.snk`, `devcon.exe`, sample APO DLLs, and source files;
3. run `InfVerif.exe /v /w <inf>` and fail on non-zero exit;
4. run `signtool verify /kp /v <cat>` when `-RequireMicrosoftSignature` is supplied;
5. return non-zero if the catalog does not cover the INF/SYS package.

- [ ] **Step 5: Run static tests**

```powershell
node --test scripts/quality/check-windows-driver-package.test.mjs
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add native/windows/driver/package scripts/windows/validate-virtual-driver-package.ps1 scripts/quality/check-windows-driver-package.test.mjs
git commit -m "feat(windows): package Voxveil virtual audio driver"
```

### Task 5: Add reproducible local driver build and attestation CAB generation

**Files:**
- Create: `scripts/windows/build-virtual-driver.ps1`
- Create: `scripts/windows/new-driver-attestation-cab.ps1`
- Modify: `package.json`

**Interfaces:**
- Produces `native/windows/driver/out/<arch>/submission/` and an attestation CAB.
- Does not sign with a private key.

- [ ] **Step 1: Implement the local build script**

Supported invocation:

```powershell
scripts/windows/build-virtual-driver.ps1 -Architecture x64 -Configuration Release
scripts/windows/build-virtual-driver.ps1 -Architecture ARM64 -Configuration Release
```

The script must locate MSBuild and WDK the same way `scripts/windows/build-windows.ps1` does, then:

1. build `VoxveilVirtualAudio.vcxproj`;
2. copy only `VoxveilVirtualAudio.sys` and `VoxveilVirtualAudio.inf` to `out/<arch>/submission`;
3. run `Inf2Cat` for the exact target OS list chosen for the release;
4. run `validate-virtual-driver-package.ps1` without `-RequireMicrosoftSignature`;
5. never invoke `bcdedit`, test certificate creation, or `signtool sign`.

- [ ] **Step 2: Implement CAB generation with DDF created in a temp directory**

`new-driver-attestation-cab.ps1 -Architecture x64 -PackageDir ... -Output ...` creates a CAB containing only the submission package at the CAB root:

```text
VoxveilVirtualAudio.inf
VoxveilVirtualAudio.sys
VoxveilVirtualAudio.cat
```

Use `makecab.exe /F <temporary.ddf>` and delete the DDF in `finally`.

- [ ] **Step 3: Add npm convenience scripts**

```json
"build:windows-driver:x64": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/build-virtual-driver.ps1 -Architecture x64 -Configuration Release",
"build:windows-driver:arm64": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/build-virtual-driver.ps1 -Architecture ARM64 -Configuration Release"
```

- [ ] **Step 4: Build both packages and inspect CAB contents**

```powershell
npm run build:windows-driver:x64
npm run build:windows-driver:arm64
expand.exe -D native/windows/driver/out/x64/VoxveilVirtualAudio-attestation-x64.cab
```

Expected: only INF/SYS/CAT entries; no keys, certificates, DevCon, DLLs, or source files.

- [ ] **Step 5: Commit**

```bash
git add scripts/windows/build-virtual-driver.ps1 scripts/windows/new-driver-attestation-cab.ps1 package.json
git commit -m "build(windows): prepare virtual driver attestation package"
```

### Task 6: Add signed-package verification and release ingestion

**Files:**
- Create: `scripts/windows/verify-signed-virtual-driver.ps1`
- Create: `scripts/windows/stage-signed-virtual-driver.ps1`
- Modify: `scripts/windows/build-windows.ps1`
- Modify: `.gitignore`

**Interfaces:**
- `VOXVEIL_SIGNED_DRIVER_DIR` is the only opt-in release input for a returned Microsoft-signed package.
- Normal app staging never consumes `out/**/submission`.

- [ ] **Step 1: Write strict verification rules**

`verify-signed-virtual-driver.ps1 -PackageDir <dir> -Architecture <x64|ARM64>` must:

```powershell
& $SignTool verify /kp /pa /v $catalog
if ($LASTEXITCODE -ne 0) { throw 'Microsoft kernel-mode catalog verification failed' }
& $SignTool verify /kp /v $sys
if ($LASTEXITCODE -ne 0) { throw 'Driver binary verification failed' }
```

It also runs `InfVerif`, confirms `Root\VoxveilVirtualAudio`, rejects test-signing strings and sample identities, and verifies the catalog/package hashes through the WDK tooling.

- [ ] **Step 2: Implement staging only after verification**

`stage-signed-virtual-driver.ps1` calls the verifier first, then copies only the verified INF/CAT/SYS into the provided destination. It must reject a destination inside `native/windows/driver/out/*/submission` to prevent confusing unsigned and signed artifacts.

- [ ] **Step 3: Gate `build-windows.ps1` release staging**

Add:

```powershell
$SignedDriverDir = $env:VOXVEIL_SIGNED_DRIVER_DIR
if ($SignedDriverDir) {
    & (Join-Path $PSScriptRoot 'stage-signed-virtual-driver.ps1') `
        -PackageDir $SignedDriverDir `
        -Architecture 'x64' `
        -Destination (Join-Path $SystemAudioStage 'virtual-driver')
    if ($LASTEXITCODE -ne 0) { throw 'Signed virtual driver staging failed.' }
}
```

When the environment variable is unset, build the app without bundling any Voxveil virtual driver.

- [ ] **Step 4: Reject unsigned/test packages in a negative test**

Run the verifier against the locally generated submission directory:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/verify-signed-virtual-driver.ps1 -PackageDir native/windows/driver/out/x64/submission -Architecture x64
```

Expected: FAIL because the package has not been returned with a trusted Microsoft production signature.

- [ ] **Step 5: Commit**

```bash
git add .gitignore scripts/windows/verify-signed-virtual-driver.ps1 scripts/windows/stage-signed-virtual-driver.ps1 scripts/windows/build-windows.ps1
git commit -m "build(windows): gate releases on signed virtual driver"
```

### Task 7: Document the Partner Center boundary and submission checklist

**Files:**
- Create: `docs/release/windows-driver-signing.md`
- Modify: `README.md`

**Interfaces:**
- Documentation distinguishes repository automation from Microsoft account/signing operations.

- [ ] **Step 1: Document prerequisites with no secrets**

The release document must state:

```text
1. Enroll the organization in Microsoft Hardware Dev Center / Partner Center.
2. Associate the required EV certificate with the account as required by current Microsoft submission rules.
3. Build the release driver CAB locally with the repository script.
4. Submit the CAB through Hardware Dev Center outside this repository.
5. Download the Microsoft-signed returned package.
6. Run verify-signed-virtual-driver.ps1.
7. Set VOXVEIL_SIGNED_DRIVER_DIR only to that verified returned package when producing the app release.
```

State explicitly that Microsoft currently describes attestation signing as a non-HLK signing option with limitations and that it does not make the package Windows Certified. Do not promise Windows Update publication through attestation.

- [ ] **Step 2: Add a release-blocking checklist**

```markdown
- [ ] Secure Boot remains enabled on the validation machine.
- [ ] TESTSIGNING is off.
- [ ] `signtool verify /kp /v` succeeds.
- [ ] INF validation succeeds.
- [ ] Fresh-machine installation succeeds without importing a local test certificate.
- [ ] Device Manager reports `Voxveil Virtual Audio` without signature errors.
- [ ] `Voxveil Input` appears as a render endpoint.
- [ ] Uninstall removes the package cleanly.
```

- [ ] **Step 3: Update README**

Describe the first-party endpoint as optional until a verified signed package is provided at release-build time; VB-CABLE remains the fallback.

- [ ] **Step 4: Commit**

```bash
git add docs/release/windows-driver-signing.md README.md
git commit -m "docs(windows): document driver signing release boundary"
```

### Task 8: Detect the signed Voxveil endpoint and reuse the Tier 1 relay

**Files:**
- Modify: `crates/voxveil-windows-audio/src/virtual_endpoint.rs`
- Modify: `crates/voxveil-windows-audio/src/relay.rs`
- Modify: `crates/voxveil-windows-audio/src/device.rs`
- Test: `crates/voxveil-windows-audio/src/virtual_endpoint.rs`
- Test: `crates/voxveil-windows-audio/src/relay.rs`

**Interfaces:**
- Produces runtime classification `VirtualEndpointKind::VoxveilCable`.
- Backend precedence becomes `loaded APO > VoxveilCableRelay > VbCableRelay`.

- [ ] **Step 1: Write classification and precedence tests**

```rust
#[test]
fn recognizes_voxveil_virtual_audio_endpoint() {
    let endpoint = endpoint("voxveil-id", "Voxveil Input", "Voxveil Virtual Audio", "Voxveil Input");
    assert_eq!(classify_virtual_endpoint(&endpoint), Some(VirtualEndpointKind::VoxveilCable));
}

#[test]
fn voxveil_cable_wins_over_vb_cable_when_apo_is_not_loaded() {
    let decision = decide_backend(
        false,
        Some(voxveil_cable(true)),
        Some(vb_cable(true)),
        Some(physical("speakers", false)),
        false,
    );
    assert_eq!(decision.interception, Some(WindowsInterceptionKind::VoxveilCableRelay));
}
```

- [ ] **Step 2: Run tests and confirm failure**

```powershell
cargo test -p voxveil-windows-audio virtual_endpoint::tests relay::tests
```

Expected: FAIL because Voxveil classification/selection is not implemented.

- [ ] **Step 3: Classify using stable Voxveil metadata**

Require both the canonical endpoint name and Voxveil device metadata:

```rust
let voxveil_name = endpoint.name.eq_ignore_ascii_case("Voxveil Input");
let voxveil_device = endpoint
    .interface_name
    .as_deref()
    .is_some_and(|value| value.eq_ignore_ascii_case("Voxveil Virtual Audio"));
if voxveil_name && voxveil_device {
    return Some(VirtualEndpointKind::VoxveilCable);
}
```

If later discovery exposes the PnP hardware ID directly, strengthen this check to require `Root\VoxveilVirtualAudio` without weakening the current fail-closed name/metadata pair.

- [ ] **Step 4: Reuse the same relay engine**

Do not create a second capture/render implementation. Construct the same `RelaySpec { source_endpoint_id, physical_output_endpoint_id }` used by VB-CABLE and set only `WindowsInterceptionKind::VoxveilCableRelay` in the route state.

- [ ] **Step 5: Run crate tests**

```powershell
cargo test -p voxveil-windows-audio
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/voxveil-windows-audio/src/virtual_endpoint.rs crates/voxveil-windows-audio/src/relay.rs crates/voxveil-windows-audio/src/device.rs
git commit -m "feat(windows): prefer signed Voxveil virtual endpoint"
```

### Task 9: Validate installation on Secure Boot production machines

**Files:**
- Create: `docs/testing/windows-signed-virtual-driver.md`

**Interfaces:**
- Produces release evidence for the exact signed package; no automated test substitutes for this hardware check.

- [ ] **Step 1: Record package identity before installation**

The test record includes:

```text
Driver package SHA-256
Catalog SHA-256
Microsoft signature verification output summary
Windows edition/build
Architecture
Secure Boot state
TESTSIGNING state
```

- [ ] **Step 2: Verify a clean install without test mode**

On a clean x64 machine and, when Arm64 is released, a clean Arm64 machine:

```powershell
Confirm-SecureBootUEFI
bcdedit /enum | Select-String testsigning
pnputil /add-driver .\VoxveilVirtualAudio.inf /install
```

Expected: Secure Boot enabled, TESTSIGNING not enabled, PnP installation succeeds with no test certificate import.

- [ ] **Step 3: Verify endpoint behavior**

Set `Voxveil Input` as the Windows default output, enable Voxveil, and confirm the existing Tier 1 relay reports `voxveil-cable-relay`, reaches `ready`, applies DSP, and renders to a distinct physical endpoint.

- [ ] **Step 4: Verify uninstall/reinstall**

Use `pnputil /enum-drivers` to locate the published INF, remove it with `/delete-driver <oemN.inf> /uninstall`, reboot if requested, confirm the endpoint is gone, then reinstall the same signed package successfully.

- [ ] **Step 5: Run repository checks**

```powershell
npm run quality
cargo test --workspace
npm run build:windows
```

Expected: PASS. The staged app includes the virtual driver only when `VOXVEIL_SIGNED_DRIVER_DIR` points to the verified returned package.

- [ ] **Step 6: Commit the test procedure**

```bash
git add docs/testing/windows-signed-virtual-driver.md
git commit -m "test(windows): document signed virtual driver validation"
```

Tier 2 is complete only after the exact package intended for release installs on a Secure Boot machine with TESTSIGNING disabled and passes the shared relay verification.