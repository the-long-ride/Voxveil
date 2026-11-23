[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Repo,
  [string]$OutputDirectory,
  [Parameter(Mandatory)][string]$Commit
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Do not compile trust anchors inherited from a driver-enabled build.
Get-ChildItem Env: | Where-Object {
  $_.Name -like 'VOXVEIL_*_SHA256'
} | ForEach-Object { Remove-Item "Env:$($_.Name)" }

Write-Host 'Building driverless Voxveil Tauri executable...'
npm run tauri -- build --no-bundle
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }
$app = Join-Path $Repo 'target\release\voxveil.exe'
if (-not (Test-Path $app -PathType Leaf)) { throw "Voxveil executable was not produced: $app" }

if (-not $OutputDirectory) {
  $OutputDirectory = Join-Path $Repo 'dist\windows-x64\Voxveil-Driverless'
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
$distRoot = Join-Path $Repo 'dist\windows-x64'
$signedInputs = @($env:VOXVEIL_SIGNED_APO_DIR, $env:VOXVEIL_SIGNED_DRIVER_DIR)
if ($env:VOXVEIL_SIGNED_DRIVER_SUBMISSION_MANIFEST) {
  $signedInputs += Split-Path -Parent ([IO.Path]::GetFullPath($env:VOXVEIL_SIGNED_DRIVER_SUBMISSION_MANIFEST))
}
Assert-SafeOutputDirectory -Output $output -RepoPath $Repo -DistRoot $distRoot -SignedInputDirectories $signedInputs
Assert-NoReparsePointInPath -Path $output -Boundary $Repo
Remove-Item $output -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $output | Out-Null
Copy-Item $app (Join-Path $output 'voxveil.exe')
& (Join-Path $PSScriptRoot 'stage-onnx-runtime.ps1') -Repo $Repo -Destination $output

@'
Voxveil Windows x64 driverless package

No APO, virtual driver, privileged installer, or system-audio helper is included.
Play local audio files directly in Voxveil, or use the automatic route:
1. Set a spare output (such as unused monitor HDMI) as the Windows default.
2. Choose a different physical output in Voxveil and enable processing.
The source must use stereo 32-bit float shared format. Do not listen to the
spare output: it carries the unprocessed audio. Two distinct outputs are required.
Visual Studio C++ Build Tools and the Windows SDK are required to build; no WDK is required.
'@ | Set-Content (Join-Path $output 'README-WINDOWS.txt') -Encoding utf8

[ordered]@{
  schemaVersion = 1
  packageVariant = 'driverless'
  voxveilCommit = $Commit
  architecture = 'x64'
  signedApo = @{ present = $false }
  signedVirtualDriver = @{ present = $false; applicability = 'not-applicable' }
  virtualDriverComponent = @{ included = $false; helperIncluded = $false; applicability = 'not-applicable' }
  packageFilesHashedBy = 'SHA256SUMS.txt'
} | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $output 'release-manifest.json') -Encoding utf8
$hashLines = Get-ChildItem $output -Recurse -File | ForEach-Object {
  "$(Get-Sha256Hex $_.FullName)  $(Get-RelativePackagePath $output $_.FullName)"
}
$hashLines | Set-Content (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii
Write-Host "Windows driverless package staged: $output"
