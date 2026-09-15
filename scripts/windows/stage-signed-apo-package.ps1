[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageDir,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Destination
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$package = [IO.Path]::GetFullPath($PackageDir)
$destination = [IO.Path]::GetFullPath($Destination)
if (-not (Test-Path $package -PathType Container)) {
  throw "Signed APO package directory not found: $package"
}
if ($destination -eq $package -or $destination.StartsWith($package + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
  throw 'Signed APO staging destination must not be the source package directory or one of its descendants.'
}

$verifier = Join-Path $PSScriptRoot 'verify-signed-apo-package.ps1'
if (-not (Test-Path $verifier -PathType Leaf)) {
  throw "Signed APO verifier not found: $verifier"
}

$verificationJson = & $verifier -PackageDir $package | Out-String
if ($LASTEXITCODE -ne 0) {
  throw 'Signed APO package verification failed.'
}
$verification = $verificationJson | ConvertFrom-Json

$expectedFiles = @(
  'VoxveilApo.inf',
  'VoxveilApo.dll',
  'VoxveilApo.cat',
  'VoxveilApoExtension.inf',
  'VoxveilApoExtension.cat'
)
$allFiles = @(Get-ChildItem $package -File -Recurse)
$resolved = [Collections.Generic.List[IO.FileInfo]]::new()
foreach ($name in $expectedFiles) {
  $matches = @($allFiles | Where-Object Name -ieq $name)
  if ($matches.Count -ne 1) {
    throw "Verified signed APO package cardinality changed unexpectedly for $name: found $($matches.Count)."
  }
  $resolved.Add($matches[0])
}

New-Item -ItemType Directory -Force -Path $destination | Out-Null
foreach ($file in $resolved) {
  Copy-Item $file.FullName (Join-Path $destination $file.Name) -Force
}

[ordered]@{
  apoInfSha256 = $verification.apoInfSha256
  apoDllSha256 = $verification.apoDllSha256
  apoCatalogSha256 = $verification.apoCatalogSha256
  extensionInfSha256 = $verification.extensionInfSha256
  extensionCatalogSha256 = $verification.extensionCatalogSha256
  apoSigner = $verification.apoSigner
  apoCatalogSigner = $verification.apoCatalogSigner
  extensionCatalogSigner = $verification.extensionCatalogSigner
  extensionId = $verification.extensionId
  capxContext = $verification.capxContext
} | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $destination 'apo-verification.json') -Encoding utf8

Write-Host "Staged verified Microsoft-signed Voxveil APO package: $destination"
