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

$package = [IO.Path]::GetFullPath($PackageDir)
$destination = [IO.Path]::GetFullPath($Destination)
$repoRoot = [IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path)
$submissionRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'native\windows\driver\out'))

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

if ($ReleaseChannel -eq 'Retail') {
  $evidencePath = Join-Path $package 'release-evidence.json'
  if (-not (Test-Path $evidencePath -PathType Leaf)) {
    throw 'Retail staging requires release-evidence.json proving the approved WHCP/HLK or Microsoft-confirmed retail signing path.'
  }
  $evidence = Get-Content $evidencePath -Raw | ConvertFrom-Json
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

Remove-Item $destination -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $destination | Out-Null
foreach ($file in @($inf, $cat, $sys)) {
  Copy-Item $file.FullName (Join-Path $destination $file.Name)
}

@{
  releaseChannel = $ReleaseChannel.ToLowerInvariant()
  architecture = $Architecture
  infSha256 = $verification.infSha256
  catalogSha256 = $verification.catalogSha256
  driverSha256 = $verification.driverSha256
  catalogSigner = $verification.catalogSigner
} | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $destination 'verification.json') -Encoding utf8

Write-Host "Staged verified $ReleaseChannel virtual driver package: $destination"
