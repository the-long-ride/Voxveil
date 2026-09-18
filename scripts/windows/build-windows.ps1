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

function Get-Sha256Hex([string]$Path) {
  $stream = [IO.File]::OpenRead($Path)
  try {
    $sha256 = [Security.Cryptography.SHA256]::Create()
    try {
      $hashBytes = $sha256.ComputeHash($stream)
      return ([BitConverter]::ToString($hashBytes)).Replace('-', '').ToLowerInvariant()
    } finally {
      $sha256.Dispose()
    }
  } finally {
    $stream.Dispose()
  }
}

function Get-NormalizedDirectoryPath([string]$Path) {
  $full = [IO.Path]::GetFullPath($Path)
  $root = [IO.Path]::GetPathRoot($full)
  if ($full -ieq $root) {
    return $root
  }
  return $full.TrimEnd([char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar))
}

function Test-DirectoryContains([string]$Parent, [string]$Child) {
  $parentFull = Get-NormalizedDirectoryPath $Parent
  $childFull = Get-NormalizedDirectoryPath $Child
  if ($parentFull -ieq $childFull) {
    return $true
  }
  $prefix = if ($parentFull.EndsWith([IO.Path]::DirectorySeparatorChar.ToString())) {
    $parentFull
  } else {
    $parentFull + [IO.Path]::DirectorySeparatorChar
  }
  return $childFull.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)
}

function Test-DirectoryOverlap([string]$Left, [string]$Right) {
  (Test-DirectoryContains $Left $Right) -or (Test-DirectoryContains $Right $Left)
}

function Assert-NoReparsePointInPath([string]$Path, [string]$Boundary) {
  $current = [IO.Path]::GetFullPath($Path)
  $boundaryFull = [IO.Path]::GetFullPath($Boundary)
  while ($true) {
    if (Test-Path -LiteralPath $current) {
      $item = Get-Item -LiteralPath $current -Force
      if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Refusing to mutate a staging/output path that traverses a junction or symbolic link: $current"
      }
    }
    if ($current -ieq $boundaryFull) {
      break
    }
    $parent = Split-Path -Parent $current
    if (-not $parent -or $parent -ieq $current) {
      throw 'Could not prove the staging/output path remains beneath the repository boundary.'
    }
    $current = $parent
  }
}

function Assert-SafeOutputDirectory(
  [string]$Output,
  [string]$RepoPath,
  [string]$DistRoot,
  [string[]]$SignedInputDirectories
) {
  $outputFull = Get-NormalizedDirectoryPath $Output
  $repoFull = Get-NormalizedDirectoryPath $RepoPath
  $distFull = Get-NormalizedDirectoryPath $DistRoot
  $volumeRoot = Get-NormalizedDirectoryPath ([IO.Path]::GetPathRoot($outputFull))

  if ($outputFull -ieq $volumeRoot -or (Test-DirectoryContains $outputFull $repoFull)) {
    throw 'Windows package output must not be a filesystem root, the repository root, or an ancestor of the repository.'
  }
  if ($outputFull -ieq $distFull -or -not (Test-DirectoryContains $distFull $outputFull)) {
    throw 'Windows package output directory must be below the repository dist\windows-x64 tree.'
  }

  foreach ($signedInput in $SignedInputDirectories) {
    if (-not $signedInput) {
      continue
    }
    if (Test-DirectoryOverlap $outputFull $signedInput) {
      throw "Windows package output must not overlap signed input directory: $signedInput"
    }
  }
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

$nativeBin = Join-Path $native 'bin\x64\Release'
$trustedDiscoveryHelper = Join-Path $repo 'scripts\windows\discover-system-audio-endpoints.ps1'
$trustedControlHelper = Join-Path $nativeBin 'voxveil-control.exe'
$trustedControlDll = Join-Path $nativeBin 'VoxveilControl.dll'
foreach ($trustedHelper in @($trustedDiscoveryHelper, $trustedControlHelper, $trustedControlDll)) {
  if (-not (Test-Path $trustedHelper -PathType Leaf)) {
    throw "Trusted privileged helper was not produced: $trustedHelper"
  }
}
$env:VOXVEIL_DISCOVERY_SHA256 = Get-Sha256Hex $trustedDiscoveryHelper
$env:VOXVEIL_CONTROL_SHA256 = Get-Sha256Hex $trustedControlHelper
$env:VOXVEIL_CONTROL_DLL_SHA256 = Get-Sha256Hex $trustedControlDll

Write-Host 'Building Voxveil Tauri executable...'
npm run tauri -- build --no-bundle
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }

$app = Join-Path $repo 'target\release\voxveil.exe'
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
$repoPath = [IO.Path]::GetFullPath($repo.Path)
$distRoot = [IO.Path]::GetFullPath((Join-Path $repoPath 'dist\windows-x64'))
$signedApoDir = $env:VOXVEIL_SIGNED_APO_DIR
$signedDriverDir = $env:VOXVEIL_SIGNED_DRIVER_DIR
Assert-SafeOutputDirectory `
  -Output $output `
  -RepoPath $repoPath `
  -DistRoot $distRoot `
  -SignedInputDirectories @($signedApoDir, $signedDriverDir)
Assert-NoReparsePointInPath -Path $output -Boundary $repoPath

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
  if (-not (Test-Path $source -PathType Leaf)) {
    throw "Required Windows package script was not found: $source"
  }
  Copy-Item $source $systemAudio
}

if ($signedApoDir) {
  & (Join-Path $PSScriptRoot 'stage-signed-apo-package.ps1') `
    -PackageDir $signedApoDir `
    -Destination $systemAudio
  if ($LASTEXITCODE -ne 0) { throw 'Signed APO package staging failed.' }
}

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
  $hash = Get-Sha256Hex $file.FullName
  $relative = Get-RelativePackagePath $output $file.FullName
  "$hash  $relative"
}
$hashLines | Set-Content (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii

Write-Host ''
Write-Host "Windows package staged: $output"
