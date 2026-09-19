[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\windows-apo',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageRoot,

  [Parameter(Mandatory = $true)]
  [ValidateSet('whcp-hlk', 'microsoft-approved-retail')]
  [string]$QualificationType,

  [Parameter(Mandatory = $true)]
  [ValidateSet('pass', 'fail', 'blocked')]
  [string]$Result,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Method,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Notes,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string[]]$EvidenceFile,

  [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-Sha256 {
  param([Parameter(Mandatory = $true)][string]$Path)
  $stream = [IO.File]::OpenRead($Path)
  try {
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
      return ([BitConverter]::ToString($hasher.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
    } finally {
      $hasher.Dispose()
    }
  } finally {
    $stream.Dispose()
  }
}

$package = [IO.Path]::GetFullPath($PackageRoot)
$releaseManifestPath = Join-Path $package 'release-manifest.json'
$apoVerificationPath = Join-Path $package 'system-audio\apo-verification.json'
foreach ($path in @($releaseManifestPath, $apoVerificationPath)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Required signed-APO package evidence was not found: $path"
  }
}

$release = Get-Content -LiteralPath $releaseManifestPath -Raw | ConvertFrom-Json
$apo = Get-Content -LiteralPath $apoVerificationPath -Raw | ConvertFrom-Json
if ($release.schemaVersion -ne 1 -or
    [string]$release.voxveilCommit -notmatch '^[a-f0-9]{40}$' -or
    [string]$release.architecture -notin @('x64', 'ARM64') -or
    -not [bool]$release.signedApo.present -or
    [string]$release.signedVirtualDriver.releaseChannel -ne 'retail') {
  throw 'APO qualification evidence requires a valid Retail final package containing a signed APO.'
}

$apoVerificationSha256 = Get-Sha256 $apoVerificationPath
if ([string]$release.signedApo.verificationSha256 -ne $apoVerificationSha256 -or
    [string]$apo.voxveilCommit -ne [string]$release.voxveilCommit) {
  throw 'Signed APO verification identity no longer matches the Retail final package.'
}
foreach ($field in @(
  'apoInfSha256',
  'apoDllSha256',
  'apoCatalogSha256',
  'extensionInfSha256',
  'extensionCatalogSha256'
)) {
  if ([string]$apo.$field -notmatch '^[a-f0-9]{64}$') {
    throw "apo-verification.json has invalid $field."
  }
}

$hashedEvidence = @()
foreach ($candidate in @($EvidenceFile)) {
  $full = [IO.Path]::GetFullPath($candidate)
  if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
    throw "APO qualification evidence artifact was not found: $full"
  }
  $item = Get-Item -LiteralPath $full
  if ($item.Length -le 0) {
    throw "APO qualification evidence artifact is empty: $full"
  }
  $hashedEvidence += [ordered]@{
    fileName = $item.Name
    sha256 = Get-Sha256 $full
    bytes = [int64]$item.Length
  }
}
if ($hashedEvidence.Count -eq 0) {
  throw 'At least one APO qualification evidence artifact is required.'
}

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$measurements = Join-Path $workspace 'measurements'
New-Item -ItemType Directory -Force -Path $measurements | Out-Null
$timestamp = [DateTime]::UtcNow
$prefix = ([string]$release.voxveilCommit).Substring(0, 12)
$output = Join-Path $measurements ("windows-apo-qualification-{0}-{1}-{2}.json" -f $QualificationType, $prefix, $timestamp.ToString('yyyyMMddTHHmmssZ'))
if ((Test-Path -LiteralPath $output) -and -not $Force) {
  throw "APO qualification evidence already exists at $output; use -Force to replace it."
}

$record = [ordered]@{
  schemaVersion = 1
  observedAtUtc = $timestamp.ToString('o')
  releaseManifestSha256 = Get-Sha256 $releaseManifestPath
  apoVerificationSha256 = $apoVerificationSha256
  voxveilCommit = [string]$release.voxveilCommit
  architecture = [string]$release.architecture
  releaseChannel = 'retail'
  package = [ordered]@{
    apoInfSha256 = [string]$apo.apoInfSha256
    apoDllSha256 = [string]$apo.apoDllSha256
    apoCatalogSha256 = [string]$apo.apoCatalogSha256
    extensionInfSha256 = [string]$apo.extensionInfSha256
    extensionCatalogSha256 = [string]$apo.extensionCatalogSha256
  }
  qualificationType = $QualificationType
  result = $Result
  method = $Method.Trim()
  notes = $Notes.Trim()
  evidenceFiles = @($hashedEvidence)
  privacy = 'Only external evidence file names, sizes, and SHA-256 values are recorded. Paths, account identifiers, credentials, and certificate private keys are not recorded.'
}

[IO.File]::WriteAllText(
  $output,
  (($record | ConvertTo-Json -Depth 10) + [Environment]::NewLine),
  [Text.UTF8Encoding]::new($false)
)

Write-Host "Windows APO qualification evidence: $Result"
Write-Host "Qualification type: $QualificationType"
Write-Host "Evidence: $output"
