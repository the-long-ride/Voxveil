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

function Assert-StagedHash {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Expected,
    [Parameter(Mandatory = $true)][string]$Label
  )

  $actual = (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $Expected.ToLowerInvariant()) {
    throw "Staged $Label hash changed after verification."
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

$package = Get-NormalizedDirectoryPath $PackageDir
$destination = Get-NormalizedDirectoryPath $Destination
$repoRoot = Get-NormalizedDirectoryPath ((Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path)
$distRoot = Get-NormalizedDirectoryPath (Join-Path $repoRoot 'dist')
if (-not (Test-Path $package -PathType Container)) {
  throw "Signed APO package directory not found: $package"
}
if (-not (Test-DirectoryContains $distRoot $destination)) {
  throw 'Signed APO staging destination must be under the repository dist tree.'
}
if (Test-DirectoryOverlap $destination $package) {
  throw 'Signed APO staging destination must not overlap the source package directory.'
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
$manifestPath = Join-Path $destination 'apo-verification.json'
Remove-Item $manifestPath -Force -ErrorAction SilentlyContinue
foreach ($file in $resolved) {
  Copy-Item $file.FullName (Join-Path $destination $file.Name) -Force
}

$stagedApoInf = Join-Path $destination 'VoxveilApo.inf'
$stagedApoDll = Join-Path $destination 'VoxveilApo.dll'
$stagedApoCat = Join-Path $destination 'VoxveilApo.cat'
$stagedExtensionInf = Join-Path $destination 'VoxveilApoExtension.inf'
$stagedExtensionCat = Join-Path $destination 'VoxveilApoExtension.cat'
Assert-StagedHash -Path $stagedApoInf -Expected $verification.apoInfSha256 -Label 'APO INF'
Assert-StagedHash -Path $stagedApoDll -Expected $verification.apoDllSha256 -Label 'APO DLL'
Assert-StagedHash -Path $stagedApoCat -Expected $verification.apoCatalogSha256 -Label 'APO catalog'
Assert-StagedHash -Path $stagedExtensionInf -Expected $verification.extensionInfSha256 -Label 'Extension INF'
Assert-StagedHash -Path $stagedExtensionCat -Expected $verification.extensionCatalogSha256 -Label 'Extension catalog'

[ordered]@{
  apoInfSha256 = $verification.apoInfSha256
  apoDllSha256 = $verification.apoDllSha256
  apoCatalogSha256 = $verification.apoCatalogSha256
  extensionInfSha256 = $verification.extensionInfSha256
  extensionCatalogSha256 = $verification.extensionCatalogSha256
  apoSigner = $verification.apoSigner
  apoThumbprint = $verification.apoThumbprint
  apoCatalogSigner = $verification.apoCatalogSigner
  apoCatalogThumbprint = $verification.apoCatalogThumbprint
  extensionCatalogSigner = $verification.extensionCatalogSigner
  extensionCatalogThumbprint = $verification.extensionCatalogThumbprint
  extensionId = $verification.extensionId
  capxContext = $verification.capxContext
} | ConvertTo-Json -Depth 3 | Set-Content $manifestPath -Encoding utf8

Write-Host "Staged verified Microsoft-signed Voxveil APO package: $destination"
