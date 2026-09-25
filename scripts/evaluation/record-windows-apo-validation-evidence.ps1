[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\windows-apo',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageRoot,

  [Parameter(Mandatory = $true)]
  [ValidateSet(
    'capx-discovery',
    'real-processing',
    'effect-state-gating',
    'raw-mode',
    'default-endpoint-handoff',
    'graph-teardown',
    'install-uninstall-coexistence',
    'reboot-resume',
    'supported-hardware-matrix'
  )]
  [string]$Scenario,

  [Parameter(Mandatory = $true)]
  [ValidateSet('pass', 'fail', 'blocked')]
  [string]$Result,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Method,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Notes,

  [string[]]$EvidenceFile = @(),

  [string]$PriorEvidence,

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

function Get-NativeArchitecture {
  $raw = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
  switch -Regex ($raw) {
    '^(?i:AMD64)$' { return 'x64' }
    '^(?i:ARM64)$' { return 'ARM64' }
    default { return $raw }
  }
}

function Get-TestSigningState {
  $systemDirectory = [Environment]::SystemDirectory
  if (-not $systemDirectory) { throw 'Windows system directory could not be resolved.' }
  $bcdedit = Join-Path $systemDirectory 'bcdedit.exe'
  if (-not (Test-Path -LiteralPath $bcdedit -PathType Leaf)) {
    throw "bcdedit.exe was not found at $bcdedit."
  }

  $output = @(& $bcdedit /enum '{current}' 2>&1)
  if ($LASTEXITCODE -ne 0) {
    throw "bcdedit.exe failed with exit code $LASTEXITCODE; TESTSIGNING cannot be proven off."
  }
  $line = @($output | Where-Object { "$_" -match '^\s*testsigning\s+' } | Select-Object -First 1)
  if ($line.Count -eq 0) {
    return [ordered]@{ rawValue = $null; enabled = $false }
  }

  $rawValue = ("$($line[0])" -replace '^\s*testsigning\s+', '').Trim()
  if ($rawValue -match '^(?i:yes|on|true|1)$') {
    return [ordered]@{ rawValue = $rawValue; enabled = $true }
  }
  if ($rawValue -match '^(?i:no|off|false|0)$') {
    return [ordered]@{ rawValue = $rawValue; enabled = $false }
  }
  throw "Unrecognized TESTSIGNING value: $rawValue"
}

function Get-PackageIdentity {
  param([Parameter(Mandatory = $true)][string]$Root)

  $package = [IO.Path]::GetFullPath($Root)
  $releaseManifestPath = Join-Path $package 'release-manifest.json'
  $apoVerificationPath = Join-Path $package 'system-audio\apo-verification.json'
  foreach ($path in @($releaseManifestPath, $apoVerificationPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
      throw "Required signed-APO package evidence was not found: $path"
    }
  }

  $release = Get-Content -LiteralPath $releaseManifestPath -Raw | ConvertFrom-Json
  $apo = Get-Content -LiteralPath $apoVerificationPath -Raw | ConvertFrom-Json
  if ($release.schemaVersion -ne 1 -or [string]$release.voxveilCommit -notmatch '^[a-f0-9]{40}$') {
    throw 'release-manifest.json has invalid release identity.'
  }
  if ([string]$release.architecture -notin @('x64', 'ARM64')) {
    throw 'release-manifest.json has invalid architecture.'
  }
  if (-not [bool]$release.signedApo.present) {
    throw 'Final package does not contain a verified signed APO.'
  }
  if ([string]$release.signedApo.verificationSha256 -notmatch '^[a-f0-9]{64}$') {
    throw 'release-manifest.json has invalid signed APO verification SHA-256.'
  }

  $apoVerificationSha256 = Get-Sha256 $apoVerificationPath
  if ($apoVerificationSha256 -ne [string]$release.signedApo.verificationSha256) {
    throw 'apo-verification.json no longer matches release-manifest.json.'
  }
  if ([string]$apo.voxveilCommit -ne [string]$release.voxveilCommit) {
    throw 'Signed APO commit does not match the final package commit.'
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

  $channel = [string]$release.signedVirtualDriver.releaseChannel
  if ($channel -notin @('pilot', 'retail')) {
    throw 'Final package does not identify a valid release channel.'
  }

  return [ordered]@{
    packageRoot = $package
    releaseManifestPath = $releaseManifestPath
    releaseManifestSha256 = Get-Sha256 $releaseManifestPath
    apoVerificationPath = $apoVerificationPath
    apoVerificationSha256 = $apoVerificationSha256
    voxveilCommit = [string]$release.voxveilCommit
    architecture = [string]$release.architecture
    releaseChannel = $channel
    apo = $apo
  }
}

$identity = Get-PackageIdentity -Root $PackageRoot

$os = Get-CimInstance Win32_OperatingSystem
[int]$windowsBuild = 0
if (-not [int]::TryParse("$($os.BuildNumber)", [ref]$windowsBuild) -or $windowsBuild -lt 22621) {
  throw 'APO production validation requires Windows build 22621 or later for the current release package.'
}
$nativeArchitecture = Get-NativeArchitecture
if ($nativeArchitecture -ne $identity.architecture) {
  throw 'Validation machine native architecture does not match the final package architecture.'
}

$secureBootCommand = Get-Command Confirm-SecureBootUEFI -ErrorAction SilentlyContinue
if (-not $secureBootCommand) {
  throw 'Confirm-SecureBootUEFI is unavailable; Secure Boot state cannot be proven.'
}
try {
  $secureBootEnabled = [bool](Confirm-SecureBootUEFI)
} catch {
  throw "Secure Boot state could not be read: $($_.Exception.GetType().FullName)"
}
if (-not $secureBootEnabled) {
  throw 'Secure Boot must be enabled for production APO validation.'
}
$testSigning = Get-TestSigningState
if ([bool]$testSigning.enabled) {
  throw 'TESTSIGNING must be off for production APO validation.'
}
if (-not $os.LastBootUpTime) {
  throw 'Current Windows boot marker could not be determined.'
}
$bootMarker = ([DateTime]$os.LastBootUpTime).ToUniversalTime().ToString('o')

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$measurements = Join-Path $workspace 'measurements'
New-Item -ItemType Directory -Force -Path $measurements | Out-Null

$priorSha256 = $null
$priorScenario = $null
$priorBootMarker = $null
if ($Scenario -eq 'reboot-resume') {
  if (-not $PriorEvidence) {
    throw 'reboot-resume evidence requires -PriorEvidence from before the restart.'
  }
  $priorPath = [IO.Path]::GetFullPath($PriorEvidence)
  $measurementPrefix = $measurements.TrimEnd('\') + '\'
  if (-not $priorPath.StartsWith($measurementPrefix, [StringComparison]::OrdinalIgnoreCase) -or
      -not (Test-Path -LiteralPath $priorPath -PathType Leaf)) {
    throw 'Prior APO evidence must be an existing file under the ignored APO measurements workspace.'
  }
  $prior = Get-Content -LiteralPath $priorPath -Raw | ConvertFrom-Json
  if ([string]$prior.releaseManifestSha256 -ne $identity.releaseManifestSha256 -or
      [string]$prior.apoVerificationSha256 -ne $identity.apoVerificationSha256 -or
      [string]$prior.result -ne 'pass') {
    throw 'Prior APO evidence is not a passing record bound to the same final package.'
  }
  $priorBootMarker = [string]$prior.machine.bootMarker
  if (-not $priorBootMarker -or $priorBootMarker -eq $bootMarker) {
    throw 'reboot-resume evidence requires a changed Windows boot marker.'
  }
  $priorSha256 = Get-Sha256 $priorPath
  $priorScenario = [string]$prior.scenario
}
elseif ($PriorEvidence) {
  throw '-PriorEvidence is only valid for reboot-resume evidence.'
}

$hashedEvidence = @()
foreach ($candidate in @($EvidenceFile)) {
  if (-not $candidate) { continue }
  $full = [IO.Path]::GetFullPath($candidate)
  if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
    throw "APO evidence attachment was not found: $full"
  }
  $item = Get-Item -LiteralPath $full
  $hashedEvidence += [ordered]@{
    fileName = $item.Name
    sha256 = Get-Sha256 $full
    bytes = [int64]$item.Length
  }
}
if ($Scenario -eq 'supported-hardware-matrix' -and $hashedEvidence.Count -eq 0) {
  throw 'supported-hardware-matrix evidence requires at least one external matrix/result artifact.'
}

$timestamp = [DateTime]::UtcNow
$prefix = $identity.voxveilCommit.Substring(0, 12)
$output = Join-Path $measurements ("windows-apo-validation-{0}-{1}-{2}.json" -f $Scenario, $prefix, $timestamp.ToString('yyyyMMddTHHmmssZ'))
if ((Test-Path -LiteralPath $output) -and -not $Force) {
  throw "APO validation evidence already exists at $output; use -Force to replace it."
}

$record = [ordered]@{
  schemaVersion = 1
  observedAtUtc = $timestamp.ToString('o')
  releaseManifestSha256 = $identity.releaseManifestSha256
  apoVerificationSha256 = $identity.apoVerificationSha256
  voxveilCommit = $identity.voxveilCommit
  architecture = $identity.architecture
  releaseChannel = $identity.releaseChannel
  package = [ordered]@{
    apoInfSha256 = [string]$identity.apo.apoInfSha256
    apoDllSha256 = [string]$identity.apo.apoDllSha256
    apoCatalogSha256 = [string]$identity.apo.apoCatalogSha256
    extensionInfSha256 = [string]$identity.apo.extensionInfSha256
    extensionCatalogSha256 = [string]$identity.apo.extensionCatalogSha256
  }
  machine = [ordered]@{
    windowsVersion = [string]$os.Version
    windowsBuild = $windowsBuild
    nativeArchitecture = $nativeArchitecture
    secureBootEnabled = $secureBootEnabled
    testSigningEnabled = [bool]$testSigning.enabled
    bootMarker = $bootMarker
  }
  scenario = $Scenario
  result = $Result
  method = $Method.Trim()
  notes = $Notes.Trim()
  priorEvidenceSha256 = $priorSha256
  priorScenario = $priorScenario
  priorBootMarker = $priorBootMarker
  evidenceFiles = @($hashedEvidence)
  privacy = 'External evidence paths, account identifiers, device serial numbers, credentials, and certificate private keys are not recorded.'
}

[IO.File]::WriteAllText(
  $output,
  (($record | ConvertTo-Json -Depth 10) + [Environment]::NewLine),
  [Text.UTF8Encoding]::new($false)
)

Write-Host "Windows APO validation evidence: $Result"
Write-Host "Scenario: $Scenario"
Write-Host "Evidence: $output"
