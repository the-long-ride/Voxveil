[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\windows-driver',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PreInstallEvidence,

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

function Assert-PathUnder {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Boundary
  )
  $full = [IO.Path]::GetFullPath($Path)
  $root = [IO.Path]::GetFullPath($Boundary)
  $prefix = if ($root.EndsWith([IO.Path]::DirectorySeparatorChar.ToString())) {
    $root
  } else {
    $root + [IO.Path]::DirectorySeparatorChar
  }
  if ($full -ieq $root -or -not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Pre-install evidence must stay beneath the ignored Windows-driver evidence workspace.'
  }
  return $full
}

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$measurements = Join-Path $workspace 'measurements'
if (-not (Test-Path -LiteralPath $measurements -PathType Container)) {
  throw "Windows-driver measurements directory was not found: $measurements"
}
$preInstallPath = Assert-PathUnder -Path $PreInstallEvidence -Boundary $measurements
if (-not (Test-Path -LiteralPath $preInstallPath -PathType Leaf)) {
  throw "Pre-install evidence was not found: $preInstallPath"
}
$preInstall = Get-Content -LiteralPath $preInstallPath -Raw | ConvertFrom-Json
if ([string]$preInstall.preInstallStatus -ne 'verified' -or
    [string]$preInstall.releaseChannel -ne 'retail') {
  throw 'Qualification evidence requires verified Retail pre-install evidence.'
}
if ([string]$preInstall.package.signingPath -ne $QualificationType) {
  throw 'Qualification type does not match the signingPath in the verified Retail pre-install evidence.'
}
if ([string]$preInstall.voxveilCommit -notmatch '^[a-f0-9]{40}$' -or
    [string]$preInstall.architecture -notin @('x64', 'ARM64')) {
  throw 'Pre-install evidence has invalid release identity.'
}
foreach ($field in @('infSha256', 'catalogSha256', 'driverSha256', 'submissionManifestSha256')) {
  if ([string]$preInstall.package.$field -notmatch '^[a-f0-9]{64}$') {
    throw "Pre-install evidence has an invalid package $field."
  }
}

$hashedEvidence = @()
foreach ($candidate in @($EvidenceFile)) {
  $full = [IO.Path]::GetFullPath($candidate)
  if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
    throw "Qualification evidence artifact was not found: $full"
  }
  $item = Get-Item -LiteralPath $full
  if ($item.Length -le 0) {
    throw "Qualification evidence artifact is empty: $full"
  }
  $hashedEvidence += [ordered]@{
    fileName = $item.Name
    sha256 = Get-Sha256 $full
    bytes = [int64]$item.Length
  }
}
if ($hashedEvidence.Count -eq 0) {
  throw 'At least one qualification evidence artifact is required.'
}

$timestamp = [DateTime]::UtcNow
$fileStamp = $timestamp.ToString('yyyyMMddTHHmmssZ')
$commitPrefix = ([string]$preInstall.voxveilCommit).Substring(0, 12)
$outputPath = Join-Path $measurements "signed-driver-qualification-$QualificationType-$commitPrefix-$fileStamp.json"
if ((Test-Path -LiteralPath $outputPath) -and -not $Force) {
  throw "Qualification evidence already exists at $outputPath; use -Force to replace it."
}

$record = [ordered]@{
  schemaVersion = 1
  observedAtUtc = $timestamp.ToString('o')
  preInstallEvidenceSha256 = Get-Sha256 $preInstallPath
  voxveilCommit = [string]$preInstall.voxveilCommit
  releaseChannel = 'retail'
  architecture = [string]$preInstall.architecture
  package = [ordered]@{
    infSha256 = [string]$preInstall.package.infSha256
    catalogSha256 = [string]$preInstall.package.catalogSha256
    driverSha256 = [string]$preInstall.package.driverSha256
    submissionManifestSha256 = [string]$preInstall.package.submissionManifestSha256
    signingPath = [string]$preInstall.package.signingPath
  }
  qualificationType = $QualificationType
  result = $Result
  method = $Method.Trim()
  notes = $Notes.Trim()
  evidenceFiles = @($hashedEvidence)
  privacy = 'Only evidence artifact file names, sizes, and SHA-256 values are recorded. Artifact paths, account identifiers, credentials, and certificate private keys are not recorded.'
}

$json = $record | ConvertTo-Json -Depth 10
[IO.File]::WriteAllText($outputPath, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))

Write-Host "Windows driver qualification evidence: $Result"
Write-Host "Qualification type: $QualificationType"
Write-Host "Evidence: $outputPath"
