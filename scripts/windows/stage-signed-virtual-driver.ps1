[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageDir,

  [ValidateSet('x64', 'ARM64')]
  [string]$Architecture = 'x64',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Destination,

  [Parameter(Mandatory = $true)]
  [ValidateSet('Pilot', 'Retail')]
  [string]$ReleaseChannel
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

$package = [IO.Path]::GetFullPath($PackageDir)
$destination = [IO.Path]::GetFullPath($Destination)
$repoRoot = [IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path)
$distRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'dist\windows-x64'))
$submissionRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'native\windows\driver\out'))

$distPrefix = if ($distRoot.EndsWith([IO.Path]::DirectorySeparatorChar.ToString())) {
  $distRoot
} else {
  $distRoot + [IO.Path]::DirectorySeparatorChar
}
if ($destination -ieq $distRoot -or
    -not $destination.StartsWith($distPrefix, [StringComparison]::OrdinalIgnoreCase)) {
  throw 'Signed virtual-driver staging destination must be below the repository dist\windows-x64 tree.'
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

$verifier = Join-Path $PSScriptRoot 'verify-signed-virtual-driver.ps1'
$verificationJson = & $verifier -PackageDir $package -Architecture $Architecture | Out-String
if ($LASTEXITCODE -ne 0) {
  throw 'Signed virtual driver verification failed.'
}
$verification = $verificationJson | ConvertFrom-Json
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
  'release-evidence.json'
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

$stagedInf = Join-Path $destination $inf.Name
$stagedCat = Join-Path $destination $cat.Name
$stagedSys = Join-Path $destination $sys.Name
Assert-StagedHash -Path $stagedInf -Expected $verification.infSha256 -Label 'INF'
Assert-StagedHash -Path $stagedCat -Expected $verification.catalogSha256 -Label 'catalog'
Assert-StagedHash -Path $stagedSys -Expected $verification.driverSha256 -Label 'driver'
if ($evidencePath) {
  Assert-StagedHash -Path (Join-Path $destination 'release-evidence.json') -Expected $releaseEvidenceSha256 -Label 'release evidence'
}

@{
  releaseChannel = $ReleaseChannel.ToLowerInvariant()
  signingPath = $verifiedSigningPath
  releaseEvidenceSha256 = $releaseEvidenceSha256
  architecture = $Architecture
  infSha256 = $verification.infSha256
  catalogSha256 = $verification.catalogSha256
  driverSha256 = $verification.driverSha256
  catalogSigner = $verification.catalogSigner
  catalogThumbprint = $verification.catalogThumbprint
} | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $destination 'verification.json') -Encoding utf8

Write-Host "Staged verified $ReleaseChannel virtual driver package: $destination"
