[CmdletBinding()]
param(
  [switch]$Force
)

$ErrorActionPreference = 'Stop'
$Revision = '67d81f217bc01edf7a4320e4911c11065635acfa'
$SysvadTreeSha = '6fa502f5bfb3de1395a6c9ffe71e322fd9e28926'
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$MetadataRoot = Join-Path $RepoRoot 'third_party\microsoft\windows-driver-samples'
$RevisionFile = Join-Path $MetadataRoot 'SOURCE_REVISION'
$TreeShaFile = Join-Path $MetadataRoot 'SYSVAD_TREE_SHA'
$Destination = Join-Path $MetadataRoot 'audio\sysvad'
$Archive = Join-Path $env:TEMP "windows-driver-samples-$Revision.zip"
$Extract = Join-Path $env:TEMP "windows-driver-samples-$Revision"
$Url = "https://github.com/microsoft/Windows-driver-samples/archive/$Revision.zip"

if (-not (Test-Path $RevisionFile -PathType Leaf)) {
  throw "Missing pinned source revision file: $RevisionFile"
}
$Pinned = (Get-Content $RevisionFile -Raw).Trim()
if ($Pinned -ne $Revision) {
  throw "SOURCE_REVISION must be exactly $Revision before importing SysVAD (found '$Pinned')."
}
if (-not (Test-Path $TreeShaFile -PathType Leaf)) {
  throw "Missing pinned SysVAD tree file: $TreeShaFile"
}
$PinnedTree = (Get-Content $TreeShaFile -Raw).Trim()
if ($PinnedTree -ne $SysvadTreeSha) {
  throw "SYSVAD_TREE_SHA must be exactly $SysvadTreeSha before importing SysVAD (found '$PinnedTree')."
}

if (Test-Path $Destination) {
  $existing = @(Get-ChildItem $Destination -Force -ErrorAction SilentlyContinue)
  if ($existing.Count -gt 0 -and -not $Force) {
    throw "Destination is not empty: $Destination. Re-run with -Force only when intentionally replacing the pinned snapshot."
  }
  if ($Force) {
    Remove-Item $Destination -Recurse -Force
  }
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null

try {
  Remove-Item $Archive -Force -ErrorAction SilentlyContinue
  Remove-Item $Extract -Recurse -Force -ErrorAction SilentlyContinue

  Write-Host "Downloading microsoft/Windows-driver-samples@$Revision (audio/sysvad tree $SysvadTreeSha)..."
  Invoke-WebRequest -Uri $Url -OutFile $Archive -UseBasicParsing

  New-Item -ItemType Directory -Force -Path $Extract | Out-Null
  Expand-Archive -Path $Archive -DestinationPath $Extract -Force

  $archiveRoot = Get-ChildItem $Extract -Directory | Select-Object -First 1
  if (-not $archiveRoot) {
    throw 'Expanded archive did not contain a repository root directory.'
  }

  $Source = Join-Path $archiveRoot.FullName 'audio\sysvad'
  if (-not (Test-Path $Source -PathType Container)) {
    throw "Pinned archive does not contain audio/sysvad: $Source"
  }

  Copy-Item $Source $Destination -Recurse -Force

  Get-ChildItem $Destination -Directory -Recurse -Force |
    Where-Object { $_.Name -in @('Debug', 'Release', '.vs', 'x64', 'ARM64') } |
    Sort-Object FullName -Descending |
    ForEach-Object { Remove-Item $_.FullName -Recurse -Force -ErrorAction SilentlyContinue }

  $forbiddenExtensions = @('.cer', '.cat', '.sys', '.dll', '.pdb', '.pfx', '.pvk', '.snk')
  Get-ChildItem $Destination -File -Recurse -Force |
    Where-Object { $forbiddenExtensions -contains $_.Extension.ToLowerInvariant() } |
    ForEach-Object { Remove-Item $_.FullName -Force }

  foreach ($required in @('README.md', 'EndpointsCommon', 'TabletAudioSample')) {
    if (-not (Test-Path (Join-Path $Destination $required))) {
      throw "Imported SysVAD snapshot is missing required path: $required"
    }
  }

  $bad = @(Get-ChildItem $Destination -File -Recurse -Force |
    Where-Object { $forbiddenExtensions -contains $_.Extension.ToLowerInvariant() })
  if ($bad.Count -gt 0) {
    throw "Imported binary/signing artifacts remain: $($bad.FullName -join ', ')"
  }

  Write-Host "Imported pinned SysVAD source to $Destination"
}
finally {
  Remove-Item $Archive -Force -ErrorAction SilentlyContinue
  Remove-Item $Extract -Recurse -Force -ErrorAction SilentlyContinue
}
