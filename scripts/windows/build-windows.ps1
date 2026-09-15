[CmdletBinding()]
param(
  [switch]$SkipNpmInstall,
  [switch]$SkipTests,
  [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = Resolve-Path (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) '..\..')
Set-Location $repo

function Find-MSBuild {
  $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
  if (Test-Path $vswhere) {
    $path = & $vswhere -latest -products * -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
    if ($path -and (Test-Path $path)) { return $path }
  }
  $command = Get-Command msbuild.exe -ErrorAction SilentlyContinue
  if ($command) { return $command.Source }
  throw 'MSBuild was not found. Install Visual Studio Build Tools with Desktop development with C++ and the Windows Driver Kit.'
}

function Assert-Wdk {
  $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Include'
  if (-not (Test-Path $kits)) {
    throw 'Windows Driver Kit headers were not found. Install the Windows 11 WDK before building VoxveilApo.dll.'
  }
  $audioHeader = Get-ChildItem $kits -Recurse -Filter 'baseaudioprocessingobject.h' -File -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $audioHeader) {
    throw 'The installed WDK does not contain baseaudioprocessingobject.h.'
  }
  $extensionHeader = Get-ChildItem $kits -Recurse -Filter 'audioengineextensionapo.h' -File -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $extensionHeader) {
    throw 'The installed WDK does not contain audioengineextensionapo.h required for IAudioSystemEffects3/CAPX.'
  }
}

function Get-RelativePackagePath([string]$BasePath, [string]$TargetPath) {
  $base = [IO.Path]::GetFullPath($BasePath)
  $separator = [IO.Path]::DirectorySeparatorChar.ToString()
  if (-not $base.EndsWith($separator)) {
    $base += $separator
  }
  $target = [IO.Path]::GetFullPath($TargetPath)
  if (-not $target.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Package file escapes staging root: $target"
  }
  return $target.Substring($base.Length).Replace('\', '/')
}

$msbuild = Find-MSBuild
Assert-Wdk

if (-not $SkipNpmInstall) {
  npm ci --ignore-scripts --no-fund --no-audit
  if ($LASTEXITCODE -ne 0) { throw "npm ci failed with exit code $LASTEXITCODE" }
}

if (-not $SkipTests) {
  cargo test -p voxveil-windows-audio
  if ($LASTEXITCODE -ne 0) { throw "voxveil-windows-audio tests failed with exit code $LASTEXITCODE" }
  npm run test:quality
  if ($LASTEXITCODE -ne 0) { throw "quality tests failed with exit code $LASTEXITCODE" }
}

$native = Join-Path $repo 'native\windows\apo'
if (-not $SkipTests) {
  $policyProject = Join-Path $native 'tests\VoxveilApoPolicyTests.vcxproj'
  Write-Host 'Building APO CAPX policy tests...'
  & $msbuild $policyProject /m /t:Rebuild /p:Configuration=Release /p:Platform=x64 /verbosity:minimal
  if ($LASTEXITCODE -ne 0) { throw "VoxveilApoPolicyTests.vcxproj failed with exit code $LASTEXITCODE" }

  $policyTests = Join-Path $native 'tests\x64\Release\VoxveilApoPolicyTests.exe'
  if (-not (Test-Path $policyTests -PathType Leaf)) {
    throw "APO policy test executable was not produced: $policyTests"
  }
  & $policyTests
  if ($LASTEXITCODE -ne 0) { throw "APO CAPX policy tests failed with exit code $LASTEXITCODE" }
}

foreach ($project in @('VoxveilControl.vcxproj', 'VoxveilControlCli.vcxproj', 'VoxveilApo.vcxproj')) {
  Write-Host "Building $project ..."
  & $msbuild (Join-Path $native $project) /m /t:Rebuild /p:Configuration=Release /p:Platform=x64 /verbosity:minimal
  if ($LASTEXITCODE -ne 0) { throw "$project failed with exit code $LASTEXITCODE" }
}

$virtualDeviceProject = Join-Path $repo 'native\windows\driver\VoxveilVirtualAudioDevice.vcxproj'
Write-Host 'Building VoxveilVirtualAudioDevice.vcxproj ...'
& $msbuild $virtualDeviceProject /m /t:Rebuild /p:Configuration=Release /p:Platform=x64 /verbosity:minimal
if ($LASTEXITCODE -ne 0) { throw "VoxveilVirtualAudioDevice.vcxproj failed with exit code $LASTEXITCODE" }

Write-Host 'Building Voxveil Tauri executable...'
npm run tauri -- build --no-bundle
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }

$app = Join-Path $repo 'target\release\voxveil.exe'
$nativeBin = Join-Path $native 'bin\x64\Release'
$requiredNative = @(
  (Join-Path $nativeBin 'VoxveilApo.dll'),
  (Join-Path $nativeBin 'VoxveilControl.dll'),
  (Join-Path $nativeBin 'voxveil-control.exe')
)
$virtualDeviceHelper = Join-Path $repo 'native\windows\driver\bin\x64\Release\voxveil-virtual-device.exe'
if (-not (Test-Path $app)) { throw "Voxveil executable was not produced: $app" }
foreach ($file in $requiredNative) {
  if (-not (Test-Path $file)) { throw "Native Windows output was not produced: $file" }
}
if (-not (Test-Path $virtualDeviceHelper -PathType Leaf)) {
  throw "Virtual audio devnode helper was not produced: $virtualDeviceHelper"
}

if (-not $OutputDirectory) {
  $OutputDirectory = Join-Path $repo 'dist\windows-x64\Voxveil'
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
$systemAudio = Join-Path $output 'system-audio'
Remove-Item $output -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $systemAudio | Out-Null

Copy-Item $app (Join-Path $output 'voxveil.exe')
Copy-Item $requiredNative -Destination $systemAudio
Copy-Item $virtualDeviceHelper (Join-Path $systemAudio 'voxveil-virtual-device.exe')
Copy-Item (Join-Path $repo 'native\windows\package\VoxveilApo.inf') $systemAudio
Copy-Item (Join-Path $repo 'native\windows\package\VoxveilApoExtension.inf.template') $systemAudio
foreach ($script in @(
  'discover-system-audio-endpoints.ps1',
  'new-apo-extension-inf.ps1',
  'install-system-audio-component.ps1',
  'uninstall-system-audio-component.ps1',
  'probe-apo-capx.ps1',
  'install-staged-virtual-driver.ps1',
  'uninstall-staged-virtual-driver.ps1'
)) {
  $source = Join-Path $repo "scripts\windows\$script"
  if (Test-Path $source -PathType Leaf) {
    Copy-Item $source $systemAudio
  }
}

$signedApoDir = $env:VOXVEIL_SIGNED_APO_DIR
if ($signedApoDir) {
  & (Join-Path $PSScriptRoot 'stage-signed-apo-package.ps1') `
    -PackageDir $signedApoDir `
    -Destination $systemAudio
  if ($LASTEXITCODE -ne 0) { throw 'Signed APO package staging failed.' }
}

$signedDriverDir = $env:VOXVEIL_SIGNED_DRIVER_DIR
if ($signedDriverDir) {
  $releaseChannel = if ($env:VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL) {
    $env:VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL
  } else {
    'Retail'
  }
  if ($releaseChannel -notin @('Pilot', 'Retail')) {
    throw 'VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL must be Pilot or Retail.'
  }
  $driverStage = Join-Path $systemAudio 'virtual-driver'
  & (Join-Path $PSScriptRoot 'stage-signed-virtual-driver.ps1') `
    -PackageDir $signedDriverDir `
    -Architecture 'x64' `
    -Destination $driverStage `
    -ReleaseChannel $releaseChannel
  if ($LASTEXITCODE -ne 0) { throw 'Signed virtual driver staging failed.' }
}

@'
Voxveil Windows x64 package

- voxveil.exe is the desktop application.
- system-audio/VoxveilApo.dll is the real in-process Windows SFX APO.
- system-audio/VoxveilControl.dll + voxveil-control.exe control and diagnose the APO.
- system-audio/voxveil-virtual-device.exe is a Voxveil-owned SetupAPI helper that creates/removes only the exact Root\VoxveilVirtualAudio devnode recorded by the signed-driver lifecycle scripts.
- Without VOXVEIL_SIGNED_APO_DIR, the staged APO DLL/INF are local development artifacts and production endpoint installation remains unavailable.
- VOXVEIL_SIGNED_APO_DIR is accepted only through verify-signed-apo-package.ps1 and stage-signed-apo-package.ps1; verified files replace the local APO DLL/INF and add the signed catalogs/Extension INF.
- Windows 11 production endpoint packages use the CAPX context property store; direct FX\0 runtime mutation is development/legacy-only.
- Voxveil resolves the selected endpoint to exact KSCATEGORY_TOPOLOGY + KSCATEGORY_AUDIO interface paths with Windows SetupAPI.
- Runtime device-interface paths are treated as opaque and are revalidated against their owning PnP instance immediately before legacy development mutation.
- The normal UI never asks for Hardware IDs or topology reference strings.
- The APO uses the Windows componentized-audio model; it does NOT install a virtual output device.
- A first-party virtual driver is staged only when VOXVEIL_SIGNED_DRIVER_DIR points to a package that passes Microsoft-signature verification.
- system-audio/install-staged-virtual-driver.ps1 first ensures exactly one Root\VoxveilVirtualAudio devnode, installs the verified driver package, and records both the exact device instance ID and published INF.
- system-audio/uninstall-staged-virtual-driver.ps1 revalidates and removes only that recorded devnode before deleting its verified driver-store package.
- Virtual-driver lifecycle tooling never enables TESTSIGNING, imports local certificates, redistributes DevCon, or performs provider-wide deletion.
- Pilot driver staging must be explicitly selected with VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL=Pilot.
- Retail driver staging defaults to Retail and requires matching WHCP/HLK or Microsoft-approved retail release evidence.
- Normal endpoint installation is offered only when the package contains a compatible production-signed CAPX Extension INF/catalog set.
- The raw -HardwareId/-ReferenceString/-TestSign parameters remain only for legacy/focused driver-development diagnostics; manual mode requires -TestSign.
'@ | Set-Content (Join-Path $output 'README-WINDOWS.txt') -Encoding utf8

$hashFiles = Get-ChildItem $output -Recurse -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' }
$hashLines = foreach ($file in $hashFiles) {
  $hash = (Get-FileHash $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  $relative = Get-RelativePackagePath $output $file.FullName
  "$hash  $relative"
}
$hashLines | Set-Content (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii

Write-Host ''
Write-Host "Windows package staged: $output"
