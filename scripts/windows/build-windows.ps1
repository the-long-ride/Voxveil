[CmdletBinding()]
param(
  [switch]$SkipNpmInstall,
  [switch]$SkipTests,
  [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = Resolve-Path (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) '..\..')
Set-Location $repo

function Find-MSBuild {
  $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
  if (Test-Path $vswhere) {
    $path = & $vswhere -latest -products * -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
    if ($path -and (Test-Path $path)) { return $path }
  }
  $command = Get-Command msbuild.exe -ErrorAction SilentlyContinue
  if ($command) { return $command.Source }
  throw 'MSBuild was not found. Install Visual Studio Build Tools with Desktop development with C++ and the Windows Driver Kit.'
}

function Assert-Wdk {
  $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Include'
  if (-not (Test-Path $kits)) {
    throw 'Windows Driver Kit headers were not found. Install the Windows 11 WDK before building VoxveilApo.dll.'
  }
  $audioHeader = Get-ChildItem $kits -Recurse -Filter 'baseaudioprocessingobject.h' -File -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $audioHeader) {
    throw 'The installed WDK does not contain baseaudioprocessingobject.h.'
  }
}

$msbuild = Find-MSBuild
Assert-Wdk

if (-not $SkipNpmInstall) {
  npm ci --ignore-scripts --no-fund --no-audit
  if ($LASTEXITCODE -ne 0) { throw "npm ci failed with exit code $LASTEXITCODE" }
}

if (-not $SkipTests) {
  cargo test -p voxveil-windows-audio
  if ($LASTEXITCODE -ne 0) { throw "voxveil-windows-audio tests failed with exit code $LASTEXITCODE" }
  npm run test:quality
  if ($LASTEXITCODE -ne 0) { throw "quality tests failed with exit code $LASTEXITCODE" }
}

$native = Join-Path $repo 'native\windows\apo'
foreach ($project in @('VoxveilControl.vcxproj', 'VoxveilControlCli.vcxproj', 'VoxveilApo.vcxproj')) {
  Write-Host "Building $project ..."
  & $msbuild (Join-Path $native $project) /m /t:Rebuild /p:Configuration=Release /p:Platform=x64 /verbosity:minimal
  if ($LASTEXITCODE -ne 0) { throw "$project failed with exit code $LASTEXITCODE" }
}

Write-Host 'Building Voxveil Tauri executable...'
npm run tauri -- build --no-bundle
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }

$app = Join-Path $repo 'target\release\voxveil.exe'
$nativeBin = Join-Path $native 'bin\x64\Release'
$requiredNative = @(
  (Join-Path $nativeBin 'VoxveilApo.dll'),
  (Join-Path $nativeBin 'VoxveilControl.dll'),
  (Join-Path $nativeBin 'voxveil-control.exe')
)
if (-not (Test-Path $app)) { throw "Voxveil executable was not produced: $app" }
foreach ($file in $requiredNative) {
  if (-not (Test-Path $file)) { throw "Native Windows output was not produced: $file" }
}

if (-not $OutputDirectory) {
  $OutputDirectory = Join-Path $repo 'dist\windows-x64\Voxveil'
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
$systemAudio = Join-Path $output 'system-audio'
Remove-Item $output -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $systemAudio | Out-Null

Copy-Item $app (Join-Path $output 'voxveil.exe')
Copy-Item $requiredNative -Destination $systemAudio
Copy-Item (Join-Path $repo 'native\windows\package\VoxveilApo.inf') $systemAudio
Copy-Item (Join-Path $repo 'native\windows\package\VoxveilApoExtension.inf.template') $systemAudio
foreach ($script in @(
  'discover-system-audio-endpoints.ps1',
  'new-apo-extension-inf.ps1',
  'install-system-audio-component.ps1',
  'uninstall-system-audio-component.ps1'
)) {
  Copy-Item (Join-Path $repo "scripts\windows\$script") $systemAudio
}

@'
Voxveil Windows x64 development package

- voxveil.exe is the desktop application.
- system-audio/VoxveilApo.dll is the real in-process Windows SFX APO.
- system-audio/VoxveilControl.dll + voxveil-control.exe control the APO and bind FX properties.
- Voxveil resolves the selected endpoint to exact KSCATEGORY_TOPOLOGY + KSCATEGORY_AUDIO interface paths with Windows SetupAPI.
- Those device-interface paths are treated as opaque and are revalidated against their owning PnP instance immediately before mutation.
- The normal runtime path does not require an OEM INF AddInterface reference string; INF reference parsing remains fallback-only.
- The normal UI never asks for Hardware IDs or topology reference strings.
- The APO uses the Windows componentized-audio model; it does NOT install a virtual output device.
- Normal endpoint installation is offered only when the package contains a compatible production-signed Extension INF/catalog set.
- The raw -HardwareId/-ReferenceString/-TestSign parameters remain only for legacy/focused driver-development diagnostics.
'@ | Set-Content (Join-Path $output 'README-WINDOWS.txt') -Encoding utf8

$hashFiles = Get-ChildItem $output -Recurse -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' }
$hashLines = foreach ($file in $hashFiles) {
  $hash = (Get-FileHash $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  $relative = [IO.Path]::GetRelativePath($output, $file.FullName).Replace('\', '/')
  "$hash  $relative"
}
$hashLines | Set-Content (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii

Write-Host ''
Write-Host "Windows package staged: $output"
Write-Host 'Next real-device validation step: verify runtime binding on physical outputs and confirm AudioDG loads VoxveilApo.dll after a compatible signed package is installed.'
