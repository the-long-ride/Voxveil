[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\windows-driver',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PreInstallEvidence,

  [Parameter(Mandatory = $true)]
  [ValidateSet('clean-install', 'reboot-resume', 'same-package-repair', 'uninstall', 'reinstall', 'package-replacement')]
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

  [string]$PriorLifecycleEvidence,

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
    [Parameter(Mandatory = $true)][string]$Boundary,
    [Parameter(Mandatory = $true)][string]$Description
  )

  $full = [IO.Path]::GetFullPath($Path)
  $root = [IO.Path]::GetFullPath($Boundary)
  $prefix = if ($root.EndsWith([IO.Path]::DirectorySeparatorChar.ToString())) {
    $root
  } else {
    $root + [IO.Path]::DirectorySeparatorChar
  }
  if ($full -ieq $root -or -not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "$Description must stay beneath the ignored Windows-driver evidence workspace."
  }
  return $full
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
  if (-not $systemDirectory) {
    throw 'Windows system directory could not be resolved.'
  }
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

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$measurements = Join-Path $workspace 'measurements'
if (-not (Test-Path -LiteralPath $measurements -PathType Container)) {
  throw "Windows-driver measurements directory was not found: $measurements"
}

$preInstallPath = Assert-PathUnder -Path $PreInstallEvidence -Boundary $measurements -Description 'Pre-install evidence'
if (-not (Test-Path -LiteralPath $preInstallPath -PathType Leaf)) {
  throw "Pre-install evidence was not found: $preInstallPath"
}
$preInstall = Get-Content -LiteralPath $preInstallPath -Raw | ConvertFrom-Json
if ([string]$preInstall.preInstallStatus -ne 'verified') {
  throw 'Pre-install evidence is not verified.'
}
if ([string]$preInstall.voxveilCommit -notmatch '^[a-f0-9]{40}$' -or
    [string]$preInstall.architecture -notin @('x64', 'ARM64') -or
    [string]$preInstall.releaseChannel -notin @('pilot', 'retail')) {
  throw 'Pre-install evidence has invalid release identity.'
}
foreach ($field in @('infSha256', 'catalogSha256', 'driverSha256', 'submissionManifestSha256')) {
  if ([string]$preInstall.package.$field -notmatch '^[a-f0-9]{64}$') {
    throw "Pre-install evidence has an invalid package $field."
  }
}
if (-not [bool]$preInstall.machine.secureBoot.enabled -or [bool]$preInstall.machine.testSigning.enabled) {
  throw 'Pre-install evidence does not prove Secure Boot enabled with TESTSIGNING off.'
}

$os = Get-CimInstance Win32_OperatingSystem
[int]$windowsBuild = 0
if (-not [int]::TryParse("$($os.BuildNumber)", [ref]$windowsBuild)) {
  throw 'Current Windows build could not be parsed.'
}
$nativeArchitecture = Get-NativeArchitecture
if ($windowsBuild -ne [int]$preInstall.machine.windowsBuild -or
    [string]$os.Version -ne [string]$preInstall.machine.windowsVersion -or
    $nativeArchitecture -ne [string]$preInstall.machine.nativeArchitecture) {
  throw 'Current Windows build/version/architecture does not match the bound pre-install evidence.'
}

$secureBootCommand = Get-Command Confirm-SecureBootUEFI -ErrorAction SilentlyContinue
if (-not $secureBootCommand) {
  throw 'Confirm-SecureBootUEFI is unavailable; Secure Boot state cannot be revalidated.'
}
try {
  $secureBootEnabled = [bool](Confirm-SecureBootUEFI)
} catch {
  throw "Secure Boot state could not be read: $($_.Exception.GetType().FullName)"
}
if (-not $secureBootEnabled) {
  throw 'Secure Boot must remain enabled while recording lifecycle evidence.'
}
$testSigning = Get-TestSigningState
if ([bool]$testSigning.enabled) {
  throw 'TESTSIGNING must remain off while recording lifecycle evidence.'
}

if (-not $os.LastBootUpTime) {
  throw 'Current Windows boot marker could not be determined.'
}
$bootMarker = ([DateTime]$os.LastBootUpTime).ToUniversalTime().ToString('o')
$preInstallSha256 = Get-Sha256 $preInstallPath

$priorSha256 = $null
$priorScenario = $null
$priorBootMarker = $null
if ($Scenario -eq 'reboot-resume') {
  if (-not $PriorLifecycleEvidence) {
    throw 'reboot-resume evidence requires -PriorLifecycleEvidence from before the restart.'
  }
  $priorPath = Assert-PathUnder -Path $PriorLifecycleEvidence -Boundary $measurements -Description 'Prior lifecycle evidence'
  if (-not (Test-Path -LiteralPath $priorPath -PathType Leaf)) {
    throw "Prior lifecycle evidence was not found: $priorPath"
  }
  $prior = Get-Content -LiteralPath $priorPath -Raw | ConvertFrom-Json
  if ([string]$prior.preInstallEvidenceSha256 -ne $preInstallSha256 -or
      [string]$prior.voxveilCommit -ne [string]$preInstall.voxveilCommit -or
      [string]$prior.result -ne 'pass') {
    throw 'Prior lifecycle evidence is not a passing record bound to the same pre-install evidence.'
  }
  $priorBootMarker = [string]$prior.machine.bootMarker
  if (-not $priorBootMarker -or $priorBootMarker -eq $bootMarker) {
    throw 'reboot-resume evidence requires a changed Windows boot marker.'
  }
  $priorSha256 = Get-Sha256 $priorPath
  $priorScenario = [string]$prior.scenario
}
elseif ($PriorLifecycleEvidence) {
  throw '-PriorLifecycleEvidence is only valid for reboot-resume evidence.'
}

$hashedEvidence = @()
foreach ($candidate in @($EvidenceFile)) {
  if (-not $candidate) { continue }
  $full = [IO.Path]::GetFullPath($candidate)
  if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
    throw "Lifecycle evidence attachment was not found: $full"
  }
  $item = Get-Item -LiteralPath $full
  $hashedEvidence += [ordered]@{
    fileName = $item.Name
    sha256 = Get-Sha256 $full
    bytes = [int64]$item.Length
  }
}

$timestamp = [DateTime]::UtcNow
$fileStamp = $timestamp.ToString('yyyyMMddTHHmmssZ')
$commitPrefix = ([string]$preInstall.voxveilCommit).Substring(0, 12)
$outputPath = Join-Path $measurements "signed-driver-lifecycle-$Scenario-$commitPrefix-$fileStamp.json"
if ((Test-Path -LiteralPath $outputPath) -and -not $Force) {
  throw "Lifecycle evidence already exists at $outputPath; use -Force to replace it."
}

$record = [ordered]@{
  schemaVersion = 1
  observedAtUtc = $timestamp.ToString('o')
  preInstallEvidenceSha256 = $preInstallSha256
  voxveilCommit = [string]$preInstall.voxveilCommit
  releaseChannel = [string]$preInstall.releaseChannel
  architecture = [string]$preInstall.architecture
  package = [ordered]@{
    infSha256 = [string]$preInstall.package.infSha256
    catalogSha256 = [string]$preInstall.package.catalogSha256
    driverSha256 = [string]$preInstall.package.driverSha256
    submissionManifestSha256 = [string]$preInstall.package.submissionManifestSha256
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
  priorLifecycleEvidenceSha256 = $priorSha256
  priorScenario = $priorScenario
  priorBootMarker = $priorBootMarker
  evidenceFiles = @($hashedEvidence)
  privacy = 'Attachment paths, user names, account identifiers, serial numbers, certificate private keys, and Partner Center credentials are not recorded.'
}

$json = $record | ConvertTo-Json -Depth 10
[IO.File]::WriteAllText($outputPath, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))

Write-Host "Windows driver lifecycle evidence: $Result"
Write-Host "Scenario: $Scenario"
Write-Host "Boot marker: $bootMarker"
Write-Host "Evidence: $outputPath"
