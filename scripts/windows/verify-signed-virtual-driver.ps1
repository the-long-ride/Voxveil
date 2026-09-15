[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageDir,

  [ValidateSet('x64', 'ARM64')]
  [string]$Architecture = 'x64'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$package = [IO.Path]::GetFullPath($PackageDir)
$validator = Join-Path $PSScriptRoot 'validate-virtual-driver-package.ps1'
if (-not (Test-Path $validator -PathType Leaf)) {
  throw "Package validator not found: $validator"
}

& $validator -PackageDir $package -Architecture $Architecture -RequireMicrosoftSignature
if ($LASTEXITCODE -ne 0) {
  throw 'Microsoft-signed virtual driver package validation failed.'
}

$infFiles = @(Get-ChildItem $package -File -Recurse -Filter '*.inf')
$catFiles = @(Get-ChildItem $package -File -Recurse -Filter '*.cat')
$sysFiles = @(Get-ChildItem $package -File -Recurse -Filter '*.sys')
if ($infFiles.Count -ne 1 -or $catFiles.Count -ne 1 -or $sysFiles.Count -ne 1) {
  throw "Verified package cardinality changed unexpectedly (INF=$($infFiles.Count), CAT=$($catFiles.Count), SYS=$($sysFiles.Count))."
}
$inf = $infFiles[0]
$cat = $catFiles[0]
$sys = $sysFiles[0]

$catalogSignature = Get-AuthenticodeSignature $cat.FullName
if ($catalogSignature.Status -ne 'Valid' -or -not $catalogSignature.SignerCertificate) {
  throw 'Catalog does not have a valid Authenticode signature.'
}
$signer = $catalogSignature.SignerCertificate
$chainText = ($signer.Subject + ' ' + $signer.Issuer)
if ($chainText -notmatch '(?i)Microsoft') {
  throw "Catalog signer is not identified as Microsoft: $($signer.Subject)"
}

$infText = Get-Content $inf.FullName -Raw
if ($infText -notmatch '(?im)Root\\VoxveilVirtualAudio') {
  throw 'Signed package INF does not match Root\VoxveilVirtualAudio.'
}
if ($infText -match '(?i)TESTSIGNING|test certificate|sysvad_|tablet audio sample|contoso') {
  throw 'Signed package contains a development/sample identity and cannot be staged.'
}

$result = [ordered]@{
  architecture = $Architecture
  packageDirectory = $package
  inf = $inf.Name
  catalog = $cat.Name
  driver = $sys.Name
  infSha256 = (Get-FileHash $inf.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  catalogSha256 = (Get-FileHash $cat.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  driverSha256 = (Get-FileHash $sys.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  catalogSigner = $signer.Subject
  catalogThumbprint = $signer.Thumbprint
}

$result | ConvertTo-Json -Depth 3
