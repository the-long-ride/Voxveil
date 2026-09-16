[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$PackageDir,

  [ValidateSet('x64', 'ARM64')]
  [string]$Architecture = 'x64',

  [switch]$SubmissionPackage,
  [switch]$RequireMicrosoftSignature
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Find-WdkTool {
  param(
    [Parameter(Mandatory = $true)][string]$Name
  )

  $command = Get-Command $Name -ErrorAction SilentlyContinue
  if ($command) { return $command.Source }

  $binRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
  if (-not (Test-Path $binRoot)) {
    throw "Windows Kits tool directory was not found while locating $Name."
  }

  $candidate = Get-ChildItem $binRoot -Directory -ErrorAction SilentlyContinue |
    Sort-Object Name -Descending |
    ForEach-Object { Join-Path $_.FullName "x64\$Name" } |
    Where-Object { Test-Path $_ } |
    Select-Object -First 1
  if ($candidate) { return $candidate }

  throw "$Name was not found in PATH or the installed Windows Kits."
}

function Get-PeMachine {
  param([Parameter(Mandatory = $true)][string]$Path)

  $stream = [IO.File]::OpenRead($Path)
  try {
    $reader = [IO.BinaryReader]::new($stream)
    try {
      if ($stream.Length -lt 64) {
        throw "Driver binary is too small to be a PE image: $Path"
      }

      $stream.Position = 0
      if ($reader.ReadUInt16() -ne 0x5A4D) {
        throw "Driver binary does not have an MZ header: $Path"
      }

      $stream.Position = 0x3C
      $peOffset = $reader.ReadInt32()
      if ($peOffset -lt 0 -or ([int64]$peOffset + 6) -gt $stream.Length) {
        throw "Driver binary has an invalid PE header offset: $Path"
      }

      $stream.Position = $peOffset
      if ($reader.ReadUInt32() -ne 0x00004550) {
        throw "Driver binary does not have a valid PE signature: $Path"
      }
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

$package = [IO.Path]::GetFullPath($PackageDir)
if (-not (Test-Path $package -PathType Container)) {
  throw "Virtual driver package directory not found: $package"
}

$files = @(Get-ChildItem $package -File -Recurse)
$inf = @($files | Where-Object Extension -ieq '.inf')
$cat = @($files | Where-Object Extension -ieq '.cat')
$sys = @($files | Where-Object Extension -ieq '.sys')
$pdb = @($files | Where-Object Extension -ieq '.pdb')

if ($inf.Count -ne 1) { throw "Package must contain exactly one INF; found $($inf.Count)." }
if ($cat.Count -ne 1) { throw "Package must contain exactly one catalog; found $($cat.Count)." }
if ($sys.Count -ne 1) { throw "Package must contain exactly one SYS; found $($sys.Count)." }
if ($SubmissionPackage -and $pdb.Count -ne 1) {
  throw "Attestation submission package must contain exactly one matching PDB; found $($pdb.Count)."
}
if (-not $SubmissionPackage -and $pdb.Count -gt 1) {
  throw "Runtime package contains unexpected PDB files: $($pdb.FullName -join ', ')"
}

$expectedMachine = if ($Architecture -eq 'ARM64') { [uint16]0xAA64 } else { [uint16]0x8664 }
$actualMachine = Get-PeMachine -Path $sys[0].FullName
if ($actualMachine -ne $expectedMachine) {
  throw ('Driver architecture mismatch: requested {0}, expected PE machine 0x{1:X4}, found 0x{2:X4}.' -f `
    $Architecture, $expectedMachine, $actualMachine)
}

$forbiddenExtensions = @('.cer', '.pfx', '.pvk', '.snk')
$forbidden = @($files | Where-Object {
  $forbiddenExtensions -contains $_.Extension.ToLowerInvariant() -or
  $_.Name -ieq 'devcon.exe' -or
  $_.Name -match '(?i)(SwapAPO|DelayAPO).*\.dll$' -or
  $_.Extension -in @('.cpp', '.c', '.h', '.hpp', '.vcxproj', '.sln')
})
if ($forbidden.Count -gt 0) {
  throw "Package contains forbidden development/signing files: $($forbidden.FullName -join ', ')"
}

$infText = Get-Content $inf[0].FullName -Raw
if ($infText -match '\$[A-Z][A-Z0-9_]*\$') {
  throw 'INF contains an unresolved WDK template token.'
}
if ($infText -notmatch '(?im)Root\\VoxveilVirtualAudio') {
  throw 'INF does not contain the production Root\VoxveilVirtualAudio hardware ID.'
}
if ($infText -match '(?i)Sysvad_|Tablet Audio Sample|Contoso|SwapAPO|DelayAPO|testsign') {
  throw 'INF contains a sample or test-signing identity that is forbidden in the Voxveil package.'
}
if ($infText -notmatch '(?im)^\s*CatalogFile\s*=\s*VoxveilVirtualAudio\.cat\s*$') {
  throw 'INF must reference VoxveilVirtualAudio.cat.'
}

$infVerif = Find-WdkTool 'InfVerif.exe'
& $infVerif /v /w $inf[0].FullName | Out-Host
if ($LASTEXITCODE -ne 0) {
  throw "InfVerif rejected $($inf[0].Name) with exit code $LASTEXITCODE."
}

if ($RequireMicrosoftSignature) {
  $signTool = Find-WdkTool 'signtool.exe'

  # PnP driver trust is established by the Microsoft-signed catalog. The SYS
  # does not need an embedded signature when its digest is covered by that
  # trusted catalog, so do not reject a valid returned package for lacking one.
  & $signTool verify /kp /v $cat[0].FullName | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw 'Microsoft kernel-mode catalog verification failed.'
  }

  foreach ($covered in @($inf[0], $sys[0])) {
    & $signTool verify /c $cat[0].FullName /v $covered.FullName | Out-Host
    if ($LASTEXITCODE -ne 0) {
      throw "Microsoft-signed catalog does not verify package member $($covered.Name)."
    }
  }
} else {
  $inf2Cat = Find-WdkTool 'Inf2Cat.exe'
  $osTarget = if ($Architecture -eq 'ARM64') { '10_ARM64' } else { '10_X64' }
  & $inf2Cat "/driver:$package" "/os:$osTarget" /verbose | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "Inf2Cat rejected the package or its referenced files (exit $LASTEXITCODE)."
  }
}

Write-Host "Virtual driver package validation passed: $package"
