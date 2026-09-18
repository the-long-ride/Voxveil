[CmdletBinding()]
param(
  [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Revision = '67d81f217bc01edf7a4320e4911c11065635acfa'
$SysvadTreeSha = '6fa502f5bfb3de1395a6c9ffe71e322fd9e28926'
$Upstream = 'https://github.com/microsoft/Windows-driver-samples.git'
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$MetadataRoot = Join-Path $RepoRoot 'third_party\microsoft\windows-driver-samples'
$RevisionFile = Join-Path $MetadataRoot 'SOURCE_REVISION'
$TreeShaFile = Join-Path $MetadataRoot 'SYSVAD_TREE_SHA'
$Destination = Join-Path $MetadataRoot 'audio\sysvad'
$CompatibilityPatcher = Join-Path $PSScriptRoot 'patch-sysvad-source.ps1'
$WorkRoot = Join-Path $env:TEMP "voxveil-windows-driver-samples-$Revision"
$Archive = Join-Path $env:TEMP "voxveil-sysvad-$Revision.zip"
$Extract = Join-Path $env:TEMP "voxveil-sysvad-$Revision"

function Invoke-GitChecked {
  param(
    [Parameter(Mandatory = $true)][string]$Git,
    [Parameter(Mandatory = $true)][string[]]$Arguments,
    [switch]$Capture
  )

  if ($Capture) {
    $output = & $Git @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
      throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE`: $($output -join [Environment]::NewLine)"
    }
    return ($output -join "`n").Trim()
  }

  & $Git @Arguments | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
  }
}

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
if (-not (Test-Path $CompatibilityPatcher -PathType Leaf)) {
  throw "Missing reviewed SysVAD compatibility patcher: $CompatibilityPatcher"
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

$gitCommand = Get-Command git.exe -ErrorAction SilentlyContinue
if (-not $gitCommand) {
  $gitCommand = Get-Command git -ErrorAction SilentlyContinue
}
if (-not $gitCommand) {
  throw 'Git was not found. Install Git before materializing the pinned SysVAD source.'
}
$Git = $gitCommand.Source

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null

try {
  Remove-Item $WorkRoot -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item $Archive -Force -ErrorAction SilentlyContinue
  Remove-Item $Extract -Recurse -Force -ErrorAction SilentlyContinue

  New-Item -ItemType Directory -Force -Path $WorkRoot | Out-Null
  Invoke-GitChecked -Git $Git -Arguments @('init', '--quiet', $WorkRoot)

  Push-Location $WorkRoot
  try {
    Invoke-GitChecked -Git $Git -Arguments @('remote', 'add', 'origin', $Upstream)
    Write-Host "Fetching microsoft/Windows-driver-samples@$Revision..."
    Invoke-GitChecked -Git $Git -Arguments @('-c', 'protocol.version=2', 'fetch', '--no-tags', '--depth', '1', 'origin', $Revision)

    $FetchedRevision = Invoke-GitChecked -Git $Git -Arguments @('rev-parse', 'FETCH_HEAD') -Capture
    if ($FetchedRevision -ne $Revision) {
      throw "Fetched Windows driver samples revision drifted: expected $Revision, found '$FetchedRevision'."
    }

    $FetchedTree = Invoke-GitChecked -Git $Git -Arguments @('rev-parse', 'FETCH_HEAD:audio/sysvad') -Capture
    if ($FetchedTree -ne $SysvadTreeSha) {
      throw "Fetched SysVAD tree identity drifted: expected $SysvadTreeSha, found '$FetchedTree'."
    }

    Invoke-GitChecked -Git $Git -Arguments @(
      'archive',
      '--format=zip',
      "--output=$Archive",
      'FETCH_HEAD',
      'audio/sysvad'
    )
  }
  finally {
    Pop-Location
  }

  if (-not (Test-Path $Archive -PathType Leaf)) {
    throw "Verified SysVAD archive was not created: $Archive"
  }

  New-Item -ItemType Directory -Force -Path $Extract | Out-Null
  Expand-Archive -Path $Archive -DestinationPath $Extract -Force

  $Source = Join-Path $Extract 'audio\sysvad'
  if (-not (Test-Path $Source -PathType Container)) {
    throw "Verified archive does not contain audio/sysvad: $Source"
  }

  Copy-Item $Source $Destination -Recurse -Force
  & $CompatibilityPatcher -SysvadRoot $Destination

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

  Write-Host "Materialized verified and Voxveil-patched SysVAD source at $Destination"
}
finally {
  Remove-Item $WorkRoot -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item $Archive -Force -ErrorAction SilentlyContinue
  Remove-Item $Extract -Recurse -Force -ErrorAction SilentlyContinue
}
