[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\windows-driver',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageDir,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$SubmissionManifest,

  [ValidateSet('x64', 'ARM64')]
  [string]$Architecture = 'x64',

  [Parameter(Mandatory = $true)]
  [ValidateSet('Pilot', 'Retail')]
  [string]$ReleaseChannel,

  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[A-Fa-f0-9]{40}$')]
  [string]$VoxveilCommit
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-Sha256 {
  param([Parameter(Mandatory = $true)][string]$Path)

  $stream = [IO.File]::OpenRead($Path)
  try {
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try {
      return ([BitConverter]::ToString($hasher.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
    }
    finally {
      $hasher.Dispose()
    }
  }
  finally {
    $stream.Dispose()
  }
}

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$git = Get-Command git.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $git) {
  throw 'git.exe is required to bind validation evidence to the exact Voxveil checkout.'
}
$currentCommit = (& $git.Source -C $repoRoot rev-parse HEAD).Trim().ToLowerInvariant()
if ($LASTEXITCODE -ne 0 -or $currentCommit -notmatch '^[a-f0-9]{40}$') {
  throw 'Could not resolve the current Voxveil commit.'
}
if ($currentCommit -ne $VoxveilCommit.ToLowerInvariant()) {
  throw "Requested Voxveil commit $VoxveilCommit does not match the current checkout $currentCommit."
}

$package = [IO.Path]::GetFullPath($PackageDir)
if (-not (Test-Path -LiteralPath $package -PathType Container)) {
  throw "Returned signed package directory was not found: $package"
}

$submissionManifestPath = [IO.Path]::GetFullPath($SubmissionManifest)
if (-not (Test-Path -LiteralPath $submissionManifestPath -PathType Leaf)) {
  throw "Unsigned submission manifest was not found: $submissionManifestPath"
}
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
$submissionManifestSha256 = Get-Sha256 $submissionManifestPath

$verifier = Join-Path $repoRoot 'scripts\windows\verify-signed-virtual-driver.ps1'
if (-not (Test-Path -LiteralPath $verifier -PathType Leaf)) {
  throw "Signed-driver verifier was not found: $verifier"
}
$verificationJson = & $verifier -PackageDir $package -Architecture $Architecture | Out-String
if ($LASTEXITCODE -ne 0) {
  throw 'Microsoft-signed virtual driver verification failed.'
}
$verification = $verificationJson | ConvertFrom-Json
if ($submission.infSha256 -ne $verification.infSha256 -or
    $submission.driverSha256 -ne $verification.driverSha256) {
  throw 'Returned Microsoft-signed package INF/SYS do not match the exact unsigned submission manifest.'
}

$releaseEvidence = $null
$releaseEvidenceSha256 = $null
if ($ReleaseChannel -eq 'Retail') {
  $releaseEvidencePath = Join-Path $package 'release-evidence.json'
  if (-not (Test-Path -LiteralPath $releaseEvidencePath -PathType Leaf)) {
    throw 'Retail validation requires release-evidence.json.'
  }
  $releaseEvidence = Get-Content -LiteralPath $releaseEvidencePath -Raw | ConvertFrom-Json
  $allowedEvidenceFields = @('releaseChannel', 'signingPath', 'infSha256', 'catalogSha256', 'driverSha256')
  $unexpected = @($releaseEvidence.PSObject.Properties.Name | Where-Object { $allowedEvidenceFields -inotcontains $_ })
  if ($unexpected.Count -gt 0) {
    throw "Retail release evidence contains undocumented fields: $($unexpected -join ', ')."
  }
  if ($releaseEvidence.releaseChannel -ne 'retail' -or
      $releaseEvidence.signingPath -notin @('whcp-hlk', 'microsoft-approved-retail')) {
    throw 'Retail release evidence does not identify an approved retail signing path.'
  }
  if ($releaseEvidence.infSha256 -ne $verification.infSha256 -or
      $releaseEvidence.catalogSha256 -ne $verification.catalogSha256 -or
      $releaseEvidence.driverSha256 -ne $verification.driverSha256) {
    throw 'Retail release-evidence hashes do not match the verified Microsoft-signed package.'
  }
  $releaseEvidenceSha256 = Get-Sha256 $releaseEvidencePath
}

$os = Get-CimInstance Win32_OperatingSystem
[int]$windowsBuild = 0
if (-not [int]::TryParse("$($os.BuildNumber)", [ref]$windowsBuild) -or $windowsBuild -lt 22621) {
  throw "Tier 2 signed-driver validation requires Windows build 22621 or later; found $($os.BuildNumber)."
}

$nativeArchitectureRaw = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
$nativeArchitecture = switch -Regex ($nativeArchitectureRaw) {
  '^(?i:AMD64)$' { 'x64'; break }
  '^(?i:ARM64)$' { 'ARM64'; break }
  default { $nativeArchitectureRaw }
}
if ($nativeArchitecture -ne $Architecture) {
  throw "Signed package architecture $Architecture does not match native Windows architecture $nativeArchitecture."
}

$secureBoot = [ordered]@{
  status = 'unavailable'
  enabled = $null
}
$secureBootCommand = Get-Command Confirm-SecureBootUEFI -ErrorAction SilentlyContinue
if (-not $secureBootCommand) {
  throw 'Confirm-SecureBootUEFI is unavailable; Secure Boot validation cannot be proven.'
}
try {
  $secureBoot.enabled = [bool](Confirm-SecureBootUEFI)
  $secureBoot.status = 'read'
}
catch {
  throw "Secure Boot state could not be read: $($_.Exception.GetType().FullName)"
}
if (-not $secureBoot.enabled) {
  throw 'Secure Boot must be enabled for signed-driver release validation.'
}

$systemDirectory = [Environment]::SystemDirectory
if (-not $systemDirectory) {
  throw 'Windows system directory could not be resolved.'
}
$bcdedit = Join-Path $systemDirectory 'bcdedit.exe'
if (-not (Test-Path -LiteralPath $bcdedit -PathType Leaf)) {
  throw "bcdedit.exe was not found at $bcdedit."
}
$bcdOutput = @(& $bcdedit /enum '{current}' 2>&1)
$bcdExitCode = $LASTEXITCODE
if ($bcdExitCode -ne 0) {
  throw "bcdedit.exe failed with exit code $bcdExitCode; TESTSIGNING cannot be proven off."
}
$testSigningLine = @($bcdOutput | Where-Object { "$_" -match '^\s*testsigning\s+' } | Select-Object -First 1)
$testSigningRaw = $null
$testSigningEnabled = $false
if ($testSigningLine.Count -gt 0) {
  $testSigningRaw = ("$($testSigningLine[0])" -replace '^\s*testsigning\s+', '').Trim()
  if ($testSigningRaw -match '^(?i:yes|on|true|1)$') {
    $testSigningEnabled = $true
  }
  elseif ($testSigningRaw -notmatch '^(?i:no|off|false|0)$') {
    throw "Unrecognized TESTSIGNING value: $testSigningRaw"
  }
}
if ($testSigningEnabled) {
  throw 'TESTSIGNING must be off for signed-driver release validation.'
}

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$measurements = Join-Path $workspace 'measurements'
New-Item -ItemType Directory -Force -Path $measurements | Out-Null
$timestamp = [DateTime]::UtcNow
$driverHashPrefix = ([string]$verification.driverSha256).Substring(0, 12)
$fileStamp = $timestamp.ToString('yyyyMMddTHHmmssZ')
$evidencePath = Join-Path $measurements "signed-driver-preinstall-$Architecture-$driverHashPrefix-$fileStamp.json"

$evidence = [ordered]@{
  generatedAtUtc = $timestamp.ToString('o')
  voxveilCommit = $currentCommit
  releaseChannel = $ReleaseChannel.ToLowerInvariant()
  architecture = $Architecture
  package = [ordered]@{
    inf = $verification.inf
    catalog = $verification.catalog
    driver = $verification.driver
    infSha256 = $verification.infSha256
    catalogSha256 = $verification.catalogSha256
    driverSha256 = $verification.driverSha256
    catalogSigner = $verification.catalogSigner
    catalogThumbprint = $verification.catalogThumbprint
    releaseEvidenceSha256 = $releaseEvidenceSha256
    submissionManifestSha256 = $submissionManifestSha256
    unsignedCatalogSha256 = $submission.catalogSha256
    unsignedPdbSha256 = $submission.pdbSha256
    signingPath = if ($releaseEvidence) { [string]$releaseEvidence.signingPath } else { 'attestation-pilot' }
  }
  machine = [ordered]@{
    windowsCaption = $os.Caption
    windowsVersion = $os.Version
    windowsBuild = $windowsBuild
    osArchitecture = $os.OSArchitecture
    nativeArchitecture = $nativeArchitecture
    secureBoot = $secureBoot
    testSigning = [ordered]@{
      bcdeditExitCode = $bcdExitCode
      rawValue = $testSigningRaw
      enabled = $testSigningEnabled
    }
  }
  preInstallStatus = 'verified'
  lifecycle = [ordered]@{
    status = 'pending'
    notes = 'Install/uninstall/reboot/repair/replacement observations require real-machine execution and are not inferred by this collector.'
  }
  privacy = 'No machine serial number, account name, user name, certificate private key, or Partner Center credential is recorded.'
}

$json = $evidence | ConvertTo-Json -Depth 10
[IO.File]::WriteAllText($evidencePath, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))

Write-Host "Signed-driver pre-install evidence: verified"
Write-Host "Voxveil commit: $currentCommit"
Write-Host "Windows build: $windowsBuild"
Write-Host "Secure Boot: $($secureBoot.enabled)"
Write-Host "TESTSIGNING: $testSigningEnabled"
Write-Host "Evidence: $evidencePath"
