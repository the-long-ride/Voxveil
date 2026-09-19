[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageDir,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$SubmissionManifest,

  [ValidateSet('x64', 'ARM64')]
  [string]$Architecture = 'x64',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Destination,

  [Parameter(Mandatory = $true)]
  [ValidateSet('Pilot', 'Retail')]
  [string]$ReleaseChannel,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$DeviceHelperPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-StagedHash {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Expected,
    [Parameter(Mandatory = $true)][string]$Label
  )

  $actual = (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $Expected) {
    throw "Staged $Label hash changed after verification."
  }
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

$package = [IO.Path]::GetFullPath($PackageDir)
$submissionManifestPath = [IO.Path]::GetFullPath($SubmissionManifest)
$destination = [IO.Path]::GetFullPath($Destination)
$repoRoot = [IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path)

if (-not (Test-Path -LiteralPath $submissionManifestPath -PathType Leaf)) {
  throw "Unsigned submission manifest was not found: $submissionManifestPath"
}
$submissionManifestSha256 = (Get-FileHash $submissionManifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
$submission = Get-Content -LiteralPath $submissionManifestPath -Raw | ConvertFrom-Json
$allowedSubmissionFields = @(
  'schemaVersion',
  'voxveilCommit',
  'architecture',
  'configuration',
  'windowsDriverSamplesRevision',
  'sysvadTreeSha',
  'infSha256',
  'catalogSha256',
  'driverSha256',
  'pdbSha256'
)
$unexpectedSubmissionFields = @(
  $submission.PSObject.Properties.Name | Where-Object { $allowedSubmissionFields -inotcontains $_ }
)
if ($unexpectedSubmissionFields.Count -gt 0) {
  throw "Unsigned submission manifest contains undocumented fields: $($unexpectedSubmissionFields -join ', ')."
}
$git = Get-Command git.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $git) {
  throw 'git.exe is required to bind signed-driver staging to the exact Voxveil checkout.'
}
$currentCommit = (& $git.Source -C $repoRoot rev-parse HEAD).Trim().ToLowerInvariant()
if ($LASTEXITCODE -ne 0 -or $currentCommit -notmatch '^[a-f0-9]{40}$') {
  throw 'Could not resolve the exact Voxveil commit for signed-driver staging.'
}
if ($submission.schemaVersion -ne 1 -or
    $submission.voxveilCommit -ne $currentCommit -or
    $submission.architecture -ne $Architecture -or
    $submission.configuration -ne 'Release' -or
    $submission.windowsDriverSamplesRevision -ne '67d81f217bc01edf7a4320e4911c11065635acfa' -or
    $submission.sysvadTreeSha -ne '6fa502f5bfb3de1395a6c9ffe71e322fd9e28926') {
  throw 'Unsigned submission manifest identity does not match the exact release checkout/architecture/pinned SysVAD provenance.'
}
foreach ($hashField in @('infSha256', 'catalogSha256', 'driverSha256', 'pdbSha256')) {
  if ([string]$submission.$hashField -notmatch '^[a-f0-9]{64}$') {
    throw "Unsigned submission manifest contains an invalid $hashField."
  }
}
$distArchitecture = if ($Architecture -eq 'ARM64') { 'windows-arm64' } else { 'windows-x64' }
$distRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot ('dist\' + $distArchitecture)))
$submissionRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'native\windows\driver\out'))
$deviceHelper = [IO.Path]::GetFullPath($DeviceHelperPath)
$expectedDeviceHelper = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $destination) 'voxveil-virtual-device.exe'))
if ($deviceHelper -ine $expectedDeviceHelper) {
  throw 'Signed virtual-driver staging must bind the SetupAPI helper packaged beside the virtual-driver directory.'
}
if (-not (Test-Path $deviceHelper -PathType Leaf)) {
  throw "Packaged Voxveil virtual-device helper was not found: $deviceHelper"
}
$deviceHelperSha256 = (Get-FileHash $deviceHelper -Algorithm SHA256).Hash.ToLowerInvariant()

$distPrefix = if ($distRoot.EndsWith([IO.Path]::DirectorySeparatorChar.ToString())) {
  $distRoot
} else {
  $distRoot + [IO.Path]::DirectorySeparatorChar
}
if ($destination -ieq $distRoot -or
    -not $destination.StartsWith($distPrefix, [StringComparison]::OrdinalIgnoreCase)) {
  throw 'Signed virtual-driver staging destination must be below the architecture-specific repository dist tree.'
}

if ($destination -eq $package -or
    $destination.StartsWith($package + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
    $package.StartsWith($destination + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
  throw 'Signed virtual-driver staging destination must not overlap the source package directory.'
}
if ($destination.StartsWith($submissionRoot, [StringComparison]::OrdinalIgnoreCase) -and
    $destination -match '(?i)[\\/]submission(?:[\\/]|$)') {
  throw 'Signed release destination must not be inside an unsigned submission directory.'
}
Assert-NoReparsePointInPath -Path $destination -Boundary $repoRoot

$verifier = Join-Path $PSScriptRoot 'verify-signed-virtual-driver.ps1'
$verificationJson = & $verifier -PackageDir $package -Architecture $Architecture | Out-String
if ($LASTEXITCODE -ne 0) {
  throw 'Signed virtual driver verification failed.'
}
$verification = $verificationJson | ConvertFrom-Json
if ($submission.infSha256 -ne $verification.infSha256 -or
    $submission.driverSha256 -ne $verification.driverSha256) {
  throw 'Returned Microsoft-signed package INF/SYS do not match the exact unsigned submission manifest.'
}
$verifiedSigningPath = 'attestation-pilot'
$releaseEvidenceSha256 = $null
$evidencePath = $null

if ($ReleaseChannel -eq 'Retail') {
  $evidencePath = Join-Path $package 'release-evidence.json'
  if (-not (Test-Path $evidencePath -PathType Leaf)) {
    throw 'Retail staging requires release-evidence.json proving the approved WHCP/HLK or Microsoft-confirmed retail signing path.'
  }
  $evidence = Get-Content $evidencePath -Raw | ConvertFrom-Json
  $allowedEvidenceFields = @('releaseChannel', 'signingPath', 'infSha256', 'catalogSha256', 'driverSha256')
  $unexpectedEvidenceFields = @(
    $evidence.PSObject.Properties.Name | Where-Object { $allowedEvidenceFields -inotcontains $_ }
  )
  if ($unexpectedEvidenceFields.Count -gt 0) {
    throw "Retail release-evidence.json contains undocumented fields that must not enter the distributable package: $($unexpectedEvidenceFields -join ', ')."
  }
  if ($evidence.releaseChannel -ne 'retail') {
    throw 'Retail release evidence must declare releaseChannel="retail".'
  }
  if ($evidence.signingPath -notin @('whcp-hlk', 'microsoft-approved-retail')) {
    throw 'Retail release evidence must use signingPath "whcp-hlk" or "microsoft-approved-retail".'
  }
  if ($evidence.infSha256 -ne $verification.infSha256 -or
      $evidence.catalogSha256 -ne $verification.catalogSha256 -or
      $evidence.driverSha256 -ne $verification.driverSha256) {
    throw 'Retail release evidence hashes do not match the verified driver package.'
  }
  $verifiedSigningPath = [string]$evidence.signingPath
  $releaseEvidenceSha256 = (Get-FileHash $evidencePath -Algorithm SHA256).Hash.ToLowerInvariant()
}

$infFiles = @(Get-ChildItem $package -File -Recurse -Filter '*.inf')
$catFiles = @(Get-ChildItem $package -File -Recurse -Filter '*.cat')
$sysFiles = @(Get-ChildItem $package -File -Recurse -Filter '*.sys')
if ($infFiles.Count -ne 1 -or $catFiles.Count -ne 1 -or $sysFiles.Count -ne 1) {
  throw "Signed package must contain exactly one INF/CAT/SYS before staging (INF=$($infFiles.Count), CAT=$($catFiles.Count), SYS=$($sysFiles.Count))."
}
$inf = $infFiles[0]
$cat = $catFiles[0]
$sys = $sysFiles[0]

$preservedInstallStateName = 'virtual-driver-install-state.json'
$existingInstallStatePath = Join-Path $destination $preservedInstallStateName
if ((Test-Path $existingInstallStatePath) -and -not (Test-Path $existingInstallStatePath -PathType Leaf)) {
  throw 'Signed virtual-driver destination contains an invalid lifecycle-state entry; refusing destructive restaging.'
}
if (Test-Path $destination -PathType Leaf) {
  throw 'Signed virtual-driver staging destination exists as a file.'
}
New-Item -ItemType Directory -Force -Path $destination | Out-Null
$allowedDestinationNames = @(
  $preservedInstallStateName,
  'VoxveilVirtualAudio.inf',
  'VoxveilVirtualAudio.cat',
  'VoxveilVirtualAudio.sys',
  'verification.json',
  'release-evidence.json',
  'submission-manifest.json'
)
$destinationEntries = @(Get-ChildItem $destination -Force)
$unexpectedDestinationEntries = @(
  $destinationEntries | Where-Object { $allowedDestinationNames -inotcontains $_.Name }
)
if ($unexpectedDestinationEntries.Count -gt 0) {
  throw "Signed virtual-driver destination contains unexpected entries; refusing destructive restaging: $($unexpectedDestinationEntries.Name -join ', ')."
}
Get-ChildItem $destination -Force |
  Where-Object { $_.Name -ine $preservedInstallStateName } |
  Remove-Item -Recurse -Force

foreach ($file in @($inf, $cat, $sys)) {
  Copy-Item $file.FullName (Join-Path $destination $file.Name)
}
if ($evidencePath) {
  Copy-Item $evidencePath (Join-Path $destination 'release-evidence.json')
}
Copy-Item $submissionManifestPath (Join-Path $destination 'submission-manifest.json')

$stagedInf = Join-Path $destination $inf.Name
$stagedCat = Join-Path $destination $cat.Name
$stagedSys = Join-Path $destination $sys.Name
Assert-StagedHash -Path $stagedInf -Expected $verification.infSha256 -Label 'INF'
Assert-StagedHash -Path $stagedCat -Expected $verification.catalogSha256 -Label 'catalog'
Assert-StagedHash -Path $stagedSys -Expected $verification.driverSha256 -Label 'driver'
if ($evidencePath) {
  Assert-StagedHash -Path (Join-Path $destination 'release-evidence.json') -Expected $releaseEvidenceSha256 -Label 'release evidence'
}
Assert-StagedHash -Path (Join-Path $destination 'submission-manifest.json') -Expected $submissionManifestSha256 -Label 'submission manifest'
Assert-StagedHash -Path $deviceHelper -Expected $deviceHelperSha256 -Label 'virtual-device helper'

@{
  releaseChannel = $ReleaseChannel.ToLowerInvariant()
  signingPath = $verifiedSigningPath
  releaseEvidenceSha256 = $releaseEvidenceSha256
  submissionManifestSha256 = $submissionManifestSha256
  voxveilCommit = $currentCommit
  unsignedCatalogSha256 = $submission.catalogSha256
  unsignedPdbSha256 = $submission.pdbSha256
  architecture = $Architecture
  deviceHelperSha256 = $deviceHelperSha256
  infSha256 = $verification.infSha256
  catalogSha256 = $verification.catalogSha256
  driverSha256 = $verification.driverSha256
  catalogSigner = $verification.catalogSigner
  catalogThumbprint = $verification.catalogThumbprint
} | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $destination 'verification.json') -Encoding utf8

Write-Host "Staged verified $ReleaseChannel virtual driver package: $destination"
