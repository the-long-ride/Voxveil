[CmdletBinding()]
param(
  [switch]$SkipNpmInstall,
  [switch]$SkipTests,
  [switch]$ApoOnly,
  [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = Resolve-Path (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) '..\..')
Set-Location $repo

function Get-TrustedProgramFilesX86 {
  $path = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFilesX86)
  if (-not $path -or -not (Test-Path $path -PathType Container)) {
    throw 'Windows Program Files (x86) directory could not be resolved from the OS known-folder API.'
  }
  return [IO.Path]::GetFullPath($path)
}

function Find-MSBuild {
  $programFilesX86 = Get-TrustedProgramFilesX86
  $vswhere = Join-Path $programFilesX86 'Microsoft Visual Studio\Installer\vswhere.exe'
  if (Test-Path $vswhere -PathType Leaf) {
    $path = & $vswhere -latest -products * -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
    if ($path -and (Test-Path $path -PathType Leaf)) { return [IO.Path]::GetFullPath($path) }
  }
  throw 'MSBuild was not found through the trusted Visual Studio Installer path. Install Visual Studio Build Tools with Desktop development with C++ and the Windows Driver Kit.'
}

function Assert-Wdk {
  $kits = Join-Path (Get-TrustedProgramFilesX86) 'Windows Kits\10\Include'
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

$git = Get-Command git.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $git) {
  throw 'git.exe is required to bind the Windows package and signed APO to the exact Voxveil checkout.'
}
$currentCommit = (& $git.Source -C $repo.Path rev-parse HEAD).Trim().ToLowerInvariant()
if ($LASTEXITCODE -ne 0 -or $currentCommit -notmatch '^[a-f0-9]{40}$') {
  throw 'Could not resolve the exact Voxveil commit for the Windows package.'
}

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
  $projectArgs = @('/m', '/t:Rebuild', '/p:Configuration=Release', '/p:Platform=x64', '/verbosity:minimal')
  if ($project -eq 'VoxveilApo.vcxproj') {
    $projectArgs += "/p:VoxveilCommit=$currentCommit"
  }
  & $msbuild (Join-Path $native $project) @projectArgs
  if ($LASTEXITCODE -ne 0) { throw "$project failed with exit code $LASTEXITCODE" }
}

if (-not $ApoOnly) {
  $virtualDeviceProject = Join-Path $repo 'native\windows\driver\VoxveilVirtualAudioDevice.vcxproj'
  Write-Host 'Building VoxveilVirtualAudioDevice.vcxproj ...'
  & $msbuild $virtualDeviceProject /m /t:Rebuild /p:Configuration=Release /p:Platform=x64 /verbosity:minimal
  if ($LASTEXITCODE -ne 0) { throw "VoxveilVirtualAudioDevice.vcxproj failed with exit code $LASTEXITCODE" }
}

$nativeBin = Join-Path $native 'bin\x64\Release'
$trustedInstallerScript = Join-Path $repo 'scripts\windows\install-system-audio-component.ps1'
$trustedDiscoveryHelper = Join-Path $repo 'scripts\windows\discover-system-audio-endpoints.ps1'
$trustedControlHelper = Join-Path $nativeBin 'voxveil-control.exe'
$trustedControlDll = Join-Path $nativeBin 'VoxveilControl.dll'
foreach ($trustedHelper in @($trustedInstallerScript, $trustedDiscoveryHelper, $trustedControlHelper, $trustedControlDll)) {
  if (-not (Test-Path $trustedHelper -PathType Leaf)) {
    throw "Trusted privileged helper was not produced: $trustedHelper"
  }
}
$trustedInstallerSha256 = Get-Sha256Hex $trustedInstallerScript
$env:VOXVEIL_DISCOVERY_SHA256 = Get-Sha256Hex $trustedDiscoveryHelper
$env:VOXVEIL_CONTROL_SHA256 = Get-Sha256Hex $trustedControlHelper
$env:VOXVEIL_CONTROL_DLL_SHA256 = Get-Sha256Hex $trustedControlDll

$signedApoDir = $env:VOXVEIL_SIGNED_APO_DIR
$apoTrustEnvNames = @(
  'VOXVEIL_APO_INF_SHA256',
  'VOXVEIL_APO_DLL_SHA256',
  'VOXVEIL_APO_CATALOG_SHA256',
  'VOXVEIL_APO_EXTENSION_INF_SHA256',
  'VOXVEIL_APO_EXTENSION_CATALOG_SHA256'
)
foreach ($name in $apoTrustEnvNames) {
  Set-Item -Path "Env:$name" -Value $null
}
if ($signedApoDir) {
  $apoVerifier = Join-Path $PSScriptRoot 'verify-signed-apo-package.ps1'
  $verifiedApoJson = (& $apoVerifier -PackageDir $signedApoDir | Out-String)
  $verifiedApo = $verifiedApoJson | ConvertFrom-Json
  if ([string]$verifiedApo.voxveilCommit -ne $currentCommit) {
    throw 'Signed APO package was not built from the exact current Voxveil checkout.'
  }
  $env:VOXVEIL_APO_INF_SHA256 = [string]$verifiedApo.apoInfSha256
  $env:VOXVEIL_APO_DLL_SHA256 = [string]$verifiedApo.apoDllSha256
  $env:VOXVEIL_APO_CATALOG_SHA256 = [string]$verifiedApo.apoCatalogSha256
  $env:VOXVEIL_APO_EXTENSION_INF_SHA256 = [string]$verifiedApo.extensionInfSha256
  $env:VOXVEIL_APO_EXTENSION_CATALOG_SHA256 = [string]$verifiedApo.extensionCatalogSha256
}

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
if (-not $ApoOnly) {
  if (-not (Test-Path $virtualDeviceHelper -PathType Leaf)) {
    throw "Virtual audio devnode helper was not produced: $virtualDeviceHelper"
  }
  $virtualDeviceHelperSha256 = Get-Sha256Hex $virtualDeviceHelper
}

if (-not $OutputDirectory) {
  $packageName = if ($ApoOnly) { 'Voxveil-Apo-Only' } else { 'Voxveil' }
  $OutputDirectory = Join-Path $repo "dist\windows-x64\$packageName"
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
$repoPath = [IO.Path]::GetFullPath($repo.Path)
$distRoot = [IO.Path]::GetFullPath((Join-Path $repoPath 'dist\windows-x64'))
$signedDriverDir = $env:VOXVEIL_SIGNED_DRIVER_DIR
$signedDriverSubmissionManifest = $env:VOXVEIL_SIGNED_DRIVER_SUBMISSION_MANIFEST
if ($ApoOnly -and ($signedDriverDir -or $signedDriverSubmissionManifest)) {
  throw 'APO-only packaging does not accept virtual-driver inputs.'
}
if ($signedDriverDir -and -not $signedDriverSubmissionManifest) {
  throw 'VOXVEIL_SIGNED_DRIVER_SUBMISSION_MANIFEST is required when VOXVEIL_SIGNED_DRIVER_DIR is set.'
}
if ($signedDriverSubmissionManifest -and -not $signedDriverDir) {
  throw 'VOXVEIL_SIGNED_DRIVER_SUBMISSION_MANIFEST must not be set without VOXVEIL_SIGNED_DRIVER_DIR.'
}
$signedDriverManifestDirectory = $null
if ($signedDriverSubmissionManifest) {
  $signedDriverSubmissionManifest = [IO.Path]::GetFullPath($signedDriverSubmissionManifest)
  if (-not (Test-Path -LiteralPath $signedDriverSubmissionManifest -PathType Leaf)) {
    throw "Signed-driver submission manifest was not found: $signedDriverSubmissionManifest"
  }
  $signedDriverManifestDirectory = Split-Path -Parent $signedDriverSubmissionManifest
}
Assert-SafeOutputDirectory `
  -Output $output `
  -RepoPath $repoPath `
  -DistRoot $distRoot `
  -SignedInputDirectories @($signedApoDir, $signedDriverDir, $signedDriverManifestDirectory)
Assert-NoReparsePointInPath -Path $output -Boundary $repoPath

$systemAudio = Join-Path $output 'system-audio'
Remove-Item $output -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $systemAudio | Out-Null

Copy-Item $app (Join-Path $output 'voxveil.exe')
Copy-Item $requiredNative -Destination $systemAudio
if (-not $ApoOnly) {
  Copy-Item $virtualDeviceHelper (Join-Path $systemAudio 'voxveil-virtual-device.exe')
}
Copy-Item (Join-Path $repo 'native\windows\package\VoxveilApo.inf') $systemAudio
Copy-Item (Join-Path $repo 'native\windows\package\VoxveilApoExtension.inf.template') $systemAudio
$packageScripts = @(
  'discover-system-audio-endpoints.ps1',
  'new-apo-extension-inf.ps1',
  'install-system-audio-component.ps1',
  'uninstall-system-audio-component.ps1',
  'probe-apo-capx.ps1'
)
if (-not $ApoOnly) {
  $packageScripts += @('install-staged-virtual-driver.ps1', 'uninstall-staged-virtual-driver.ps1')
}
foreach ($script in $packageScripts) {
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

$releaseChannel = $null
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
    -SubmissionManifest $signedDriverSubmissionManifest `
    -Architecture 'x64' `
    -Destination $driverStage `
    -ReleaseChannel $releaseChannel `
    -DeviceHelperPath (Join-Path $systemAudio 'voxveil-virtual-device.exe')
  if ($LASTEXITCODE -ne 0) { throw 'Signed virtual driver staging failed.' }
}

$trustedPackageFiles = @(
  @{
    Path = Join-Path $systemAudio 'install-system-audio-component.ps1'
    Expected = $trustedInstallerSha256
  },
  @{
    Path = Join-Path $systemAudio 'discover-system-audio-endpoints.ps1'
    Expected = $env:VOXVEIL_DISCOVERY_SHA256
  },
  @{
    Path = Join-Path $systemAudio 'voxveil-control.exe'
    Expected = $env:VOXVEIL_CONTROL_SHA256
  },
  @{
    Path = Join-Path $systemAudio 'VoxveilControl.dll'
    Expected = $env:VOXVEIL_CONTROL_DLL_SHA256
  }
)
if (-not $ApoOnly) {
  $trustedPackageFiles += @{
    Path = Join-Path $systemAudio 'voxveil-virtual-device.exe'
    Expected = $virtualDeviceHelperSha256
  }
}
if ($signedApoDir) {
  $trustedPackageFiles += @(
    @{
      Path = Join-Path $systemAudio 'VoxveilApo.inf'
      Expected = $env:VOXVEIL_APO_INF_SHA256
    },
    @{
      Path = Join-Path $systemAudio 'VoxveilApo.dll'
      Expected = $env:VOXVEIL_APO_DLL_SHA256
    },
    @{
      Path = Join-Path $systemAudio 'VoxveilApo.cat'
      Expected = $env:VOXVEIL_APO_CATALOG_SHA256
    },
    @{
      Path = Join-Path $systemAudio 'VoxveilApoExtension.inf'
      Expected = $env:VOXVEIL_APO_EXTENSION_INF_SHA256
    },
    @{
      Path = Join-Path $systemAudio 'VoxveilApoExtension.cat'
      Expected = $env:VOXVEIL_APO_EXTENSION_CATALOG_SHA256
    }
  )
}
foreach ($trustedPackageFile in $trustedPackageFiles) {
  $actual = Get-Sha256Hex $trustedPackageFile.Path
  if ($actual -ne $trustedPackageFile.Expected) {
    throw "Packaged trusted file changed after Tauri trust anchors were compiled: $($trustedPackageFile.Path)"
  }
}

$readmeText = @'
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
- A first-party virtual driver is staged only when VOXVEIL_SIGNED_DRIVER_DIR points to a Microsoft-verified package and VOXVEIL_SIGNED_DRIVER_SUBMISSION_MANIFEST points to the retained exact-SHA unsigned submission manifest.
- system-audio/install-staged-virtual-driver.ps1 first ensures exactly one Root\VoxveilVirtualAudio devnode, installs the verified driver package, and records both the exact device instance ID and published INF.
- system-audio/uninstall-staged-virtual-driver.ps1 revalidates and removes only that recorded devnode before deleting its verified driver-store package.
- Virtual-driver lifecycle tooling never enables TESTSIGNING, imports local certificates, redistributes DevCon, or performs provider-wide deletion.
- Pilot driver staging must be explicitly selected with VOXVEIL_SIGNED_DRIVER_RELEASE_CHANNEL=Pilot.
- Retail driver staging defaults to Retail and requires matching WHCP/HLK or Microsoft-approved retail release evidence.
- release-manifest.json records the exact Voxveil commit plus signed APO/virtual-driver presence and is itself covered by SHA256SUMS.txt.
- Normal endpoint installation is offered only when the package contains a compatible production-signed CAPX Extension INF/catalog set.
- The raw -HardwareId/-ReferenceString/-TestSign parameters remain only for legacy/focused driver-development diagnostics; manual mode requires -TestSign.
'@
if ($ApoOnly) {
  $readmeText = $readmeText -replace '(?m)^- system-audio/voxveil-virtual-device\.exe.*\r?\n', ''
  $readmeText = $readmeText -replace '(?m)^- A first-party virtual driver is staged only.*\r?\n', ''
  $readmeText = $readmeText -replace '(?m)^- Virtual-driver lifecycle tooling never.*\r?\n', ''
  $readmeText = $readmeText -replace '(?m)^- Pilot driver staging must be.*\r?\n', ''
  $readmeText = $readmeText -replace '(?m)^- Retail driver staging defaults.*\r?\n', ''
  $readmeText += "`n- Package variant: physical APO only. No virtual-driver package, helper, installer, or lifecycle scripts are included.`n- The virtual-driver component is not applicable to this package variant.`n- The physical APO still installs a signed Windows audio component on a compatible endpoint; this is not a no-install system-wide route.`n"
}
$readmeText | Set-Content (Join-Path $output 'README-WINDOWS.txt') -Encoding utf8

$apoVerificationPath = Join-Path $systemAudio 'apo-verification.json'
$driverVerificationPath = Join-Path $systemAudio 'virtual-driver\verification.json'
$releaseManifest = [ordered]@{
  schemaVersion = 1
  packageVariant = if ($ApoOnly) { 'apo-only' } else { 'full' }
  voxveilCommit = $currentCommit
  architecture = 'x64'
  signedApo = [ordered]@{
    present = [bool]$signedApoDir
    verificationSha256 = if (Test-Path -LiteralPath $apoVerificationPath -PathType Leaf) { Get-Sha256Hex $apoVerificationPath } else { $null }
  }
  signedVirtualDriver = [ordered]@{
    present = (-not $ApoOnly) -and [bool]$signedDriverDir
    applicability = if ($ApoOnly) { 'not-applicable' } else { 'optional' }
    releaseChannel = if ($releaseChannel) { $releaseChannel.ToLowerInvariant() } else { $null }
    verificationSha256 = if (Test-Path -LiteralPath $driverVerificationPath -PathType Leaf) { Get-Sha256Hex $driverVerificationPath } else { $null }
  }
  virtualDriverComponent = [ordered]@{
    included = (-not $ApoOnly) -and [bool]$signedDriverDir
    applicability = if ($ApoOnly) { 'not-applicable' } else { 'optional' }
    helperIncluded = -not $ApoOnly
  }
  packageFilesHashedBy = 'SHA256SUMS.txt'
}
$releaseManifest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $output 'release-manifest.json') -Encoding utf8

$hashFiles = Get-ChildItem $output -Recurse -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' }
$hashLines = foreach ($file in $hashFiles) {
  $hash = Get-Sha256Hex $file.FullName
  $relative = Get-RelativePackagePath $output $file.FullName
  "$hash  $relative"
}
$hashLines | Set-Content (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii

Write-Host ''
Write-Host "Windows package staged: $output"
