[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageDir
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-TrustedProgramFilesX86 {
  $path = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFilesX86)
  if (-not $path -or -not (Test-Path $path -PathType Container)) {
    throw 'Windows Program Files (x86) directory could not be resolved from the OS known-folder API.'
  }
  return [IO.Path]::GetFullPath($path)
}

function Find-WdkTool {
  param([Parameter(Mandatory = $true)][string]$Name)

  $binRoot = Join-Path (Get-TrustedProgramFilesX86) 'Windows Kits\10\bin'
  if (-not (Test-Path $binRoot -PathType Container)) {
    throw "Windows Kits tool directory was not found while locating $Name."
  }
  $toolArchitectures = @('x64', 'x86')

  foreach ($toolArchitecture in $toolArchitectures) {
    $direct = Join-Path $binRoot "$toolArchitecture\$Name"
    if (Test-Path $direct -PathType Leaf) { return $direct }
  }
  foreach ($versionDirectory in @(Get-ChildItem $binRoot -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending)) {
    foreach ($toolArchitecture in $toolArchitectures) {
      $candidate = Join-Path $versionDirectory.FullName "$toolArchitecture\$Name"
      if (Test-Path $candidate -PathType Leaf) { return $candidate }
    }
  }
  throw "$Name was not found in the trusted Windows Kits bin directory."
}

function Get-ExactlyOneFile {
  param(
    [Parameter(Mandatory = $true)][IO.FileInfo[]]$Files,
    [Parameter(Mandatory = $true)][string]$Name
  )

  $matches = @($Files | Where-Object Name -ieq $Name)
  if ($matches.Count -ne 1) {
    throw "Signed APO package must contain exactly one $Name; found $($matches.Count)."
  }
  return $matches[0]
}

function Get-PeMachine {
  param([Parameter(Mandatory = $true)][string]$Path)

  $stream = [IO.File]::OpenRead($Path)
  try {
    $reader = [IO.BinaryReader]::new($stream)
    try {
      if ($stream.Length -lt 64) { throw "PE image is too small: $Path" }
      if ($reader.ReadUInt16() -ne 0x5A4D) { throw "PE image has no MZ header: $Path" }
      $stream.Position = 0x3C
      $peOffset = $reader.ReadInt32()
      if ($peOffset -lt 0 -or ([int64]$peOffset + 6) -gt $stream.Length) {
        throw "PE image has an invalid header offset: $Path"
      }
      $stream.Position = $peOffset
      if ($reader.ReadUInt32() -ne 0x00004550) { throw "PE image has no valid PE signature: $Path" }
      return [uint16]$reader.ReadUInt16()
    }
    finally {
      $reader.Dispose()
    }
  }
  finally {
    $stream.Dispose()
  }
}

function Assert-MicrosoftSignature {
  param(
    [Parameter(Mandatory = $true)][IO.FileInfo]$File,
    [Parameter(Mandatory = $true)][string]$Description
  )

  $signature = Get-AuthenticodeSignature $File.FullName
  if ($signature.Status -ne 'Valid' -or -not $signature.SignerCertificate) {
    throw "$Description does not have a valid Authenticode signature: $($File.FullName)"
  }
  $identity = "$($signature.SignerCertificate.Subject) $($signature.SignerCertificate.Issuer)"
  if ($identity -notmatch '(?i)Microsoft') {
    throw "$Description is not identified as Microsoft-signed: $($signature.SignerCertificate.Subject)"
  }
  [pscustomobject]@{
    Subject = [string]$signature.SignerCertificate.Subject
    Thumbprint = [string]$signature.SignerCertificate.Thumbprint
  }
}

$package = [IO.Path]::GetFullPath($PackageDir)
if (-not (Test-Path $package -PathType Container)) {
  throw "Signed APO package directory not found: $package"
}

$files = @(Get-ChildItem $package -File -Recurse)
$apoInf = Get-ExactlyOneFile -Files $files -Name 'VoxveilApo.inf'
$apoDll = Get-ExactlyOneFile -Files $files -Name 'VoxveilApo.dll'
$apoCat = Get-ExactlyOneFile -Files $files -Name 'VoxveilApo.cat'
$extensionInf = Get-ExactlyOneFile -Files $files -Name 'VoxveilApoExtension.inf'
$extensionCat = Get-ExactlyOneFile -Files $files -Name 'VoxveilApoExtension.cat'

$unexpectedPackageFiles = @($files | Where-Object {
  $_.Extension -in @('.inf', '.cat', '.dll') -and
  $_.Name -notin @(
    'VoxveilApo.inf',
    'VoxveilApo.dll',
    'VoxveilApo.cat',
    'VoxveilApoExtension.inf',
    'VoxveilApoExtension.cat'
  )
})
if ($unexpectedPackageFiles.Count -gt 0) {
  throw "Signed APO package contains ambiguous INF/CAT/DLL files: $($unexpectedPackageFiles.FullName -join ', ')"
}

$forbidden = @($files | Where-Object {
  $_.Extension.ToLowerInvariant() -in @('.cer', '.pfx', '.pvk', '.snk', '.cpp', '.c', '.h', '.hpp') -or
  $_.Name -match '(?i)VoxveilDevelopment|testcert|private.?key'
})
if ($forbidden.Count -gt 0) {
  throw "Signed APO package contains development/signing material: $($forbidden.FullName -join ', ')"
}

if ((Get-PeMachine -Path $apoDll.FullName) -ne [uint16]0x8664) {
  throw 'VoxveilApo.dll driver architecture must be x64 for the current desktop release.'
}

$apoText = Get-Content $apoInf.FullName -Raw
if ($apoText -notmatch '(?im)^\s*Class\s*=\s*AudioProcessingObject\s*$') {
  throw 'VoxveilApo.inf must use Class=AudioProcessingObject for the Windows 11 package.'
}
if ($apoText -notmatch '(?im)^\s*CatalogFile\s*=\s*VoxveilApo\.cat\s*$') {
  throw 'VoxveilApo.inf must reference VoxveilApo.cat.'
}
if ($apoText -notmatch '(?i)F3F2A99F-8FB7-4B88-949E-448BF8A05221') {
  throw 'VoxveilApo.inf does not contain the fixed Voxveil SFX CLSID.'
}
if ($apoText -notmatch '(?im)^\s*VoxveilApo\.dll\s*=\s*SignatureAttributes\.PETrust\s*$') {
  throw 'VoxveilApo.inf must request PETrust for VoxveilApo.dll.'
}

$extensionText = Get-Content $extensionInf.FullName -Raw
if ($extensionText -notmatch '(?im)^\s*Class\s*=\s*Extension\s*$') {
  throw 'VoxveilApoExtension.inf must use Class=Extension.'
}
if ($extensionText -notmatch '(?im)^\s*CatalogFile\s*=\s*VoxveilApoExtension\.cat\s*$') {
  throw 'VoxveilApoExtension.inf must reference VoxveilApoExtension.cat.'
}
if ($extensionText -notmatch '(?im)^\s*ExtensionId\s*=\s*\{1D81E93D-AB81-473B-9E5E-94FAE8D2377F\}\s*$') {
  throw 'VoxveilApoExtension.inf does not match the committed Voxveil extension servicing lineage.'
}
if ($extensionText -notmatch '(?i)VOXVEIL_APO_CONTEXT\s*=\s*"\{63E268CE-4CBC-48E0-BEB6-55103316F477\}"') {
  throw 'VoxveilApoExtension.inf does not contain the fixed CAPX property context.'
}
if ($extensionText -notmatch '(?im)^\s*HKR\s*,\s*FX\\0\\%VOXVEIL_APO_CONTEXT%\s*,\s*%PKEY_FX_Association%') {
  throw 'VoxveilApoExtension.inf is not CAPX-bound to the fixed property context.'
}
if ($extensionText -match '(?im)^\s*HKR\s*,\s*FX\\0\s*,\s*%PKEY_FX_Association%') {
  throw 'Production VoxveilApoExtension.inf must not contain the legacy root FX association.'
}
if ($extensionText -notmatch '(?im)^\s*AddInterface\s*=') {
  throw 'Production VoxveilApoExtension.inf must bind through signed endpoint interfaces.'
}

$infVerif = Find-WdkTool 'InfVerif.exe'
foreach ($inf in @($apoInf, $extensionInf)) {
  & $infVerif /v /w $inf.FullName | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "InfVerif rejected $($inf.Name) with exit code $LASTEXITCODE."
  }
}

$signTool = Find-WdkTool 'signtool.exe'
foreach ($catalog in @($apoCat, $extensionCat)) {
  & $signTool verify /kp /v $catalog.FullName | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "Microsoft driver-catalog verification failed for $($catalog.Name)."
  }
}
foreach ($member in @($apoInf, $apoDll)) {
  & $signTool verify /c $apoCat.FullName /v $member.FullName | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "VoxveilApo.cat does not verify package member $($member.Name)."
  }
}
& $signTool verify /c $extensionCat.FullName /v $extensionInf.FullName | Out-Host
if ($LASTEXITCODE -ne 0) {
  throw 'VoxveilApoExtension.cat does not verify VoxveilApoExtension.inf.'
}

# The INF requests SignatureAttributes.PETrust for this user-mode APO binary.
# Require a valid Microsoft Authenticode signature in addition to catalog membership.
& $signTool verify /pa /v $apoDll.FullName | Out-Host
if ($LASTEXITCODE -ne 0) {
  throw 'VoxveilApo.dll PETrust/Authenticode verification failed.'
}
$apoSignature = Assert-MicrosoftSignature -File $apoDll -Description 'VoxveilApo.dll'
$apoCatalogSignature = Assert-MicrosoftSignature -File $apoCat -Description 'VoxveilApo.cat'
$extensionCatalogSignature = Assert-MicrosoftSignature -File $extensionCat -Description 'VoxveilApoExtension.cat'

[ordered]@{
  packageDirectory = $package
  apoInfSha256 = (Get-FileHash $apoInf.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  apoDllSha256 = (Get-FileHash $apoDll.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  apoCatalogSha256 = (Get-FileHash $apoCat.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  extensionInfSha256 = (Get-FileHash $extensionInf.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  extensionCatalogSha256 = (Get-FileHash $extensionCat.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  apoSigner = $apoSignature.Subject
  apoThumbprint = $apoSignature.Thumbprint
  apoCatalogSigner = $apoCatalogSignature.Subject
  apoCatalogThumbprint = $apoCatalogSignature.Thumbprint
  extensionCatalogSigner = $extensionCatalogSignature.Subject
  extensionCatalogThumbprint = $extensionCatalogSignature.Thumbprint
  extensionId = '1D81E93D-AB81-473B-9E5E-94FAE8D2377F'
  capxContext = '63E268CE-4CBC-48E0-BEB6-55103316F477'
} | ConvertTo-Json -Depth 3
