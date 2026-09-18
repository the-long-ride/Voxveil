[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$SysvadRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = [IO.Path]::GetFullPath($SysvadRoot)
$target = Join-Path $root 'EndpointsCommon\MiniportAudioEngineNode.cpp'
$marker = '// Voxveil compatibility: guard upstream sideband-only branch for the non-sideband render build.'
$guard = '#if defined(SYSVAD_BTH_BYPASS) || defined(SYSVAD_USB_SIDEBAND)'
$guardEnd = '#endif // defined(SYSVAD_BTH_BYPASS) || defined(SYSVAD_USB_SIDEBAND)'
$sidebandIfPattern = '^\s*if \(IsSidebandDevice\(\) && m_pSidebandDevice->Is(?:Volume|Mute)Supported\(m_DeviceType\)\)\s*$'

if (-not (Test-Path $target -PathType Leaf)) {
  throw "Pinned SysVAD compatibility target is missing: $target"
}

$lines = [IO.File]::ReadAllLines($target)
if (@($lines | Where-Object { $_ -eq $marker }).Count -gt 0) {
  throw 'Pinned SysVAD compatibility patch was already applied; refusing a second mutation.'
}

$output = [System.Collections.Generic.List[string]]::new()
$patchedCount = 0

for ($i = 0; $i -lt $lines.Length; $i++) {
  $line = $lines[$i]
  if ($line -notmatch $sidebandIfPattern) {
    $output.Add($line)
    continue
  }

  $patchedCount++
  $output.Add($marker)
  $output.Add($guard)
  $output.Add($line)

  $depth = 0
  $sawOpeningBrace = $false
  $closed = $false

  while (++$i -lt $lines.Length) {
    $current = $lines[$i]
    $output.Add($current)

    $openCount = ([regex]::Matches($current, '\{')).Count
    $closeCount = ([regex]::Matches($current, '\}')).Count
    if ($openCount -gt 0) {
      $sawOpeningBrace = $true
    }
    $depth += $openCount - $closeCount

    if ($sawOpeningBrace -and $depth -eq 0) {
      if (($i + 1) -lt $lines.Length -and $lines[$i + 1].Trim() -eq 'else') {
        $i++
        $output.Add($lines[$i])
      }
      $output.Add($guardEnd)
      $closed = $true
      break
    }
  }

  if (-not $closed) {
    throw "Pinned SysVAD compatibility patch could not find the end of sideband branch $patchedCount."
  }
}

if ($patchedCount -ne 8) {
  throw "Pinned SysVAD compatibility patch expected exactly 8 sideband volume/mute branches, found $patchedCount."
}

[IO.File]::WriteAllLines($target, $output, [Text.UTF8Encoding]::new($false))
Write-Host "Applied Voxveil non-sideband compatibility guards to $patchedCount pinned SysVAD engine-node branches."
