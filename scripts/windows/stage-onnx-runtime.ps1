[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Repo,
  [Parameter(Mandatory)][string]$Destination
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$url = 'https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/1.23.2/microsoft.ml.onnxruntime.1.23.2.nupkg'
$expected = '25fe172f7fcdf34f2b5b02c8b997f6b88abc282956849d4af4335af23d0c4e4a'
$cache = Join-Path $Repo 'target\onnxruntime-1.23.2.nupkg'
if (-not (Test-Path -LiteralPath $Destination -PathType Container)) {
  throw 'ONNX Runtime staging requires an existing package directory.'
}
if (-not (Test-Path -LiteralPath $cache -PathType Leaf)) {
  Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $cache
}
if ((Get-FileHash -LiteralPath $cache -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
  throw 'ONNX Runtime package SHA-256 mismatch; remove the cached package and retry.'
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($cache)
try {
  $files = @{
    'runtimes/win-x64/native/onnxruntime.dll' = 'onnxruntime.dll'
    'runtimes/win-x64/native/onnxruntime_providers_shared.dll' = 'onnxruntime_providers_shared.dll'
    'LICENSE' = 'ONNX_RUNTIME_LICENSE.txt'
    'ThirdPartyNotices.txt' = 'ONNX_RUNTIME_ThirdPartyNotices.txt'
  }
  foreach ($name in $files.Keys) {
    $entry = $archive.GetEntry($name)
    if (-not $entry) { throw "ONNX Runtime package is missing $name" }
    [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $Destination $files[$name]), $true)
  }
} finally {
  $archive.Dispose()
}
Copy-Item (Join-Path $Repo 'THIRD_PARTY_NOTICES.md') (Join-Path $Destination 'THIRD_PARTY_NOTICES.md')
