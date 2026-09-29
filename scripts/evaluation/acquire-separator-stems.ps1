[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\separator',
  [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$allowedHosts = @('ccmixter.org', 'zenodo.org', 'datadryad.org', 'upload.wikimedia.org')
$allowedLicenses = @('CC0', 'CC BY 3.0', 'CC BY 4.0')

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$sources = Join-Path $workspace 'sources'
$manifestPath = Join-Path $workspace 'manifests\stems.json'
if (-not (Test-Path $manifestPath -PathType Leaf)) {
  throw "Manifest not found: $manifestPath. Copy the example from docs/testing/separator-stem-corpus.md and fill it in."
}
New-Item -ItemType Directory -Force -Path $sources | Out-Null
$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json

foreach ($stem in $manifest.stems) {
  $uri = [Uri]$stem.url
  if ($uri.Scheme -ne 'https') { throw "$($stem.id): only https URLs are allowed" }
  $hostOk = $false
  foreach ($allowed in $allowedHosts) {
    if ($uri.Host -eq $allowed -or $uri.Host.EndsWith(".$allowed")) { $hostOk = $true }
  }
  if (-not $hostOk) { throw "$($stem.id): host $($uri.Host) is not on the allowlist" }
  if ($allowedLicenses -notcontains $stem.license) { throw "$($stem.id): license '$($stem.license)' is not allowed" }
  if ($stem.role -ne 'vocal' -and $stem.role -ne 'instrumental') { throw "$($stem.id): role must be vocal or instrumental" }

  $extension = [IO.Path]::GetExtension($uri.AbsolutePath)
  $target = Join-Path $sources "$($stem.id)$extension"
  if ($Force -or -not (Test-Path $target -PathType Leaf)) {
    Write-Host "Downloading $($stem.id) from $($uri.Host)..."
    Invoke-WebRequest -Uri $uri -OutFile "$target.download" -UseBasicParsing
    Move-Item "$target.download" $target -Force
  }
  $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash.ToLowerInvariant()
  if ([string]::IsNullOrEmpty($stem.sha256)) {
    $stem.sha256 = $hash
    Write-Host "$($stem.id): pinned sha256 $hash"
  } elseif ($stem.sha256 -ne $hash) {
    throw "$($stem.id): sha256 mismatch (manifest $($stem.sha256), file $hash)"
  }
}

$manifest | ConvertTo-Json -Depth 5 | Set-Content -Path $manifestPath -Encoding utf8
Write-Host "All stems verified under $sources"
