[CmdletBinding()]
param(
  [ValidateSet('x64', 'ARM64')]
  [string]$Architecture = 'x64',

  [ValidateSet('Debug', 'Release')]
  [string]$Configuration = 'Release'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$DriverRoot = Join-Path $RepoRoot 'native\windows\driver'
$Project = Join-Path $DriverRoot 'VoxveilVirtualAudio.vcxproj'
$InfSource = Join-Path $DriverRoot 'package\VoxveilVirtualAudio.inf'
$SysvadRoot = Join-Path $RepoRoot 'third_party\microsoft\windows-driver-samples\audio\sysvad'
$PinnedRevisionFile = Join-Path $RepoRoot 'third_party\microsoft\windows-driver-samples\SOURCE_REVISION'
$PinnedTreeFile = Join-Path $RepoRoot 'third_party\microsoft\windows-driver-samples\SYSVAD_TREE_SHA'
$PinnedRevision = '67d81f217bc01edf7a4320e4911c11065635acfa'
$PinnedSysvadTree = '6fa502f5bfb3de1395a6c9ffe71e322fd9e28926'
$BuildOutput = Join-Path $DriverRoot "build\$Architecture\$Configuration"
$DriverOutput = Join-Path $BuildOutput 'VoxveilVirtualAudio.sys'
$PdbOutput = Join-Path $BuildOutput 'VoxveilVirtualAudio.pdb'
$OutRoot = Join-Path $DriverRoot "out\$Architecture"
$Submission = Join-Path $OutRoot 'submission'
$SysvadImporter = Join-Path $PSScriptRoot 'import-sysvad-source.ps1'

function Find-MSBuild {
  $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
  if (Test-Path $vswhere) {
    $path = & $vswhere -latest -products * -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
    if ($path -and (Test-Path $path)) { return $path }
  }
  $command = Get-Command msbuild.exe -ErrorAction SilentlyContinue
  if ($command) { return $command.Source }
  throw 'MSBuild was not found. Install Visual Studio Build Tools with C++ and the Windows Driver Kit.'
}

function Find-WdkTool {
  param([Parameter(Mandatory = $true)][string]$Name)

  $command = Get-Command $Name -ErrorAction SilentlyContinue
  if ($command) { return $command.Source }

  $searchRoots = @(
    (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'),
    (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Tools')
  )

  foreach ($root in $searchRoots) {
    if (-not (Test-Path $root -PathType Container)) { continue }

    $direct = Join-Path $root "x64\$Name"
    if (Test-Path $direct -PathType Leaf) { return $direct }

    $candidate = Get-ChildItem $root -Directory -ErrorAction SilentlyContinue |
      Sort-Object Name -Descending |
      ForEach-Object { Join-Path $_.FullName "x64\$Name" } |
      Where-Object { Test-Path $_ -PathType Leaf } |
      Select-Object -First 1
    if ($candidate) { return $candidate }
  }

  throw "$Name was not found in PATH or the installed Windows Kits bin/Tools directories."
}

foreach ($required in @($Project, $InfSource, $PinnedRevisionFile, $PinnedTreeFile, $SysvadImporter)) {
  if (-not (Test-Path $required -PathType Leaf)) {
    throw "Required driver source/package file is missing: $required"
  }
}

$revision = (Get-Content $PinnedRevisionFile -Raw).Trim()
if ($revision -ne $PinnedRevision) {
  throw "Windows driver samples revision drifted: expected $PinnedRevision, found '$revision'."
}
$sysvadTree = (Get-Content $PinnedTreeFile -Raw).Trim()
if ($sysvadTree -ne $PinnedSysvadTree) {
  throw "SysVAD tree identity drifted: expected $PinnedSysvadTree, found '$sysvadTree'."
}

Write-Host 'Materializing verified pinned SysVAD source...'
& $SysvadImporter -Force
if ($LASTEXITCODE -ne 0) {
  throw "SysVAD importer failed with exit code $LASTEXITCODE."
}

if (-not (Test-Path $SysvadRoot -PathType Container)) {
  throw "Verified SysVAD source was not materialized at $SysvadRoot."
}
foreach ($requiredSource in @(
  'adapter.cpp',
  'common.cpp',
  'EndpointsCommon\minwavert.cpp',
  'EndpointsCommon\minwavertstream.cpp',
  'EndpointsCommon\mintopo.cpp'
)) {
  if (-not (Test-Path (Join-Path $SysvadRoot $requiredSource) -PathType Leaf)) {
    throw "Verified SysVAD snapshot is incomplete: missing $requiredSource."
  }
}

$projectText = Get-Content $Project -Raw
if (($projectText | Select-String -Pattern '<SignMode>Off</SignMode>' -AllMatches).Matches.Count -ne 4) {
  throw 'VoxveilVirtualAudio.vcxproj must keep SignMode=Off for all x64/ARM64 Debug/Release configurations.'
}
if ($projectText -match '(?i)TestSign|Test Certificate|\.pfx|\.cer') {
  throw 'Virtual driver project contains test-signing material, which is forbidden.'
}

$msbuild = Find-MSBuild
$null = Find-WdkTool 'Inf2Cat.exe'

Write-Host "Building unsigned Voxveil virtual audio driver: $Configuration|$Architecture"
& $msbuild $Project /m /t:Rebuild "/p:Configuration=$Configuration" "/p:Platform=$Architecture" /verbosity:minimal
if ($LASTEXITCODE -ne 0) {
  throw "VoxveilVirtualAudio.vcxproj failed with exit code $LASTEXITCODE."
}

if (-not (Test-Path $DriverOutput -PathType Leaf)) {
  throw "Driver build did not produce the expected architecture/configuration output: $DriverOutput"
}
if (-not (Test-Path $PdbOutput -PathType Leaf)) {
  throw "Driver build did not produce the matching PDB required for submission: $PdbOutput"
}

Remove-Item $Submission -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Submission | Out-Null
Copy-Item $DriverOutput (Join-Path $Submission 'VoxveilVirtualAudio.sys')
Copy-Item $PdbOutput (Join-Path $Submission 'VoxveilVirtualAudio.pdb')
Copy-Item $InfSource (Join-Path $Submission 'VoxveilVirtualAudio.inf')

$inf2Cat = Find-WdkTool 'Inf2Cat.exe'
$osTarget = if ($Architecture -eq 'ARM64') { '10_ARM64' } else { '10_X64' }
& $inf2Cat "/driver:$Submission" "/os:$osTarget" /verbose | Out-Host
if ($LASTEXITCODE -ne 0) {
  throw "Inf2Cat failed with exit code $LASTEXITCODE."
}

$validator = Join-Path $PSScriptRoot 'validate-virtual-driver-package.ps1'
& $validator -PackageDir $Submission -Architecture $Architecture -SubmissionPackage
if ($LASTEXITCODE -ne 0) {
  throw "Virtual driver package validation failed with exit code $LASTEXITCODE."
}

Write-Host "Unsigned virtual-driver submission package staged at $Submission"
