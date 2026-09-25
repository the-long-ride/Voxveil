[CmdletBinding()]
param(
  [ValidateSet('x64', 'ARM64')]
  [string]$Architecture = 'x64',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageDir,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Output
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$package = [IO.Path]::GetFullPath($PackageDir)
$outputPath = [IO.Path]::GetFullPath($Output)
if (-not (Test-Path $package -PathType Container)) {
  throw "Package directory not found: $package"
}

$required = @(
  'VoxveilVirtualAudio.inf',
  'VoxveilVirtualAudio.sys',
  'VoxveilVirtualAudio.cat',
  'VoxveilVirtualAudio.pdb'
)
foreach ($name in $required) {
  if (-not (Test-Path (Join-Path $package $name) -PathType Leaf)) {
    throw "Attestation package is missing required file: $name"
  }
}

$trustedSystemDirectory = [Environment]::SystemDirectory
if (-not $trustedSystemDirectory) {
  throw 'Windows system directory could not be resolved for makecab.exe.'
}
$makecab = Join-Path $trustedSystemDirectory 'makecab.exe'
if (-not (Test-Path $makecab -PathType Leaf)) {
  throw "makecab.exe was not found at $makecab."
}

$outputDir = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
Remove-Item $outputPath -Force -ErrorAction SilentlyContinue

$tempRoot = Join-Path $env:TEMP ("voxveil-driver-cab-" + [Guid]::NewGuid().ToString('N'))
$ddf = Join-Path $tempRoot 'VoxveilVirtualAudio.ddf'
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

try {
  $cabName = Split-Path -Leaf $outputPath
  $lines = @(
    '.OPTION EXPLICIT',
    ".Set CabinetNameTemplate=`"$cabName`"",
    ".Set DiskDirectoryTemplate=`"$outputDir`"",
    '.Set Cabinet=on',
    '.Set Compress=on',
    '.Set CompressionType=MSZIP',
    '.Set DestinationDir=VoxveilVirtualAudio'
  )
  foreach ($name in $required) {
    $source = Join-Path $package $name
    $lines += '"' + $source + '" "' + $name + '"'
  }
  $lines | Set-Content $ddf -Encoding ascii

  & $makecab /F $ddf | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "makecab.exe failed with exit code $LASTEXITCODE."
  }
  if (-not (Test-Path $outputPath -PathType Leaf)) {
    throw "makecab.exe did not produce the expected CAB: $outputPath"
  }
}
finally {
  Remove-Item $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Created attestation CAB for $Architecture: $outputPath"
Write-Host 'The CAB contains VoxveilVirtualAudio/{INF,SYS,CAT,PDB}.'
Write-Host 'Before Hardware Dev Center upload, Authenticode-sign this CAB externally with the approved EV-certificate process and SHA-256 timestamping.'
