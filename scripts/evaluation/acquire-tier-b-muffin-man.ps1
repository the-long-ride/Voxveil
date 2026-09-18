[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\classic-dsp',
  [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$sourceUrl = 'https://upload.wikimedia.org/wikipedia/commons/f/fc/The_Muffin_Man_%28performed_by_Sinclair_Ukiri%29.ogg'
$sourceRecord = 'https://commons.wikimedia.org/wiki/File:The_Muffin_Man_(performed_by_Sinclair_Ukiri).ogg'
$fileName = 'The_Muffin_Man_(performed_by_Sinclair_Ukiri).ogg'
$expectedBytes = 1101160L
$expectedSha1 = '0a2ff6ab77db7995c4a8b2ed868520bd80838574'

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$sources = Join-Path $workspace 'sources'
$manifests = Join-Path $workspace 'manifests'
$sourcePath = Join-Path $sources $fileName
$tempPath = "$sourcePath.download"
$manifestPath = Join-Path $manifests 'natural-mix-sinclair-ukiri-muffin-man-001.json'

New-Item -ItemType Directory -Force -Path $sources, $manifests | Out-Null

if ($Force -or -not (Test-Path $sourcePath -PathType Leaf)) {
  if (Test-Path $tempPath -PathType Leaf) {
    Remove-Item $tempPath -Force
  }
  try {
    Write-Host "Downloading approved Tier-B source from Wikimedia Commons..."
    Invoke-WebRequest -Uri $sourceUrl -OutFile $tempPath -UseBasicParsing
    Move-Item $tempPath $sourcePath -Force
  }
  finally {
    if (Test-Path $tempPath -PathType Leaf) {
      Remove-Item $tempPath -Force
    }
  }
}
else {
  Write-Host "Using existing source file: $sourcePath"
}

$item = Get-Item $sourcePath
if ($item.Length -ne $expectedBytes) {
  throw "Tier-B source size mismatch: expected $expectedBytes bytes, got $($item.Length)."
}

$sha1 = (Get-FileHash $sourcePath -Algorithm SHA1).Hash.ToLowerInvariant()
if ($sha1 -ne $expectedSha1) {
  throw "Tier-B source SHA-1 mismatch: expected $expectedSha1, got $sha1."
}

$sha256 = (Get-FileHash $sourcePath -Algorithm SHA256).Hash.ToLowerInvariant()
$licenseCheckedOn = (Get-Date).ToString('yyyy-MM-dd')

$manifest = [ordered]@{
  fixtureId = 'natural-mix-sinclair-ukiri-muffin-man-001'
  tier = 'natural-mix'
  status = 'approved-metadata'
  targetSampleRate = 44100
  durationSeconds = 19.6875
  sources = @(
    [ordered]@{
      role = 'mixed'
      title = 'The Muffin Man (performed by Sinclair Ukiri)'
      sourceRecord = $sourceRecord
      sourceFile = "sources/$fileName"
      license = 'CC-BY-4.0'
      licenseCheckedOn = $licenseCheckedOn
      sourceSha256 = $sha256
    }
  )
  mixRecipe = $null
  fixtureSha256 = $sha256
  notes = 'Original licensed source acquired unchanged; metadata approved, listening/evaluation still pending.'
}

$json = $manifest | ConvertTo-Json -Depth 8
[IO.File]::WriteAllText($manifestPath, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))

Write-Host "Verified SHA-1: $sha1"
Write-Host "Local SHA-256: $sha256"
Write-Host "Manifest: $manifestPath"
