[CmdletBinding()]
param(
  [string]$ControlPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $ControlPath) {
  $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
  $candidates = @(
    (Join-Path $scriptDir 'voxveil-control.exe'),
    (Join-Path $scriptDir '..\..\native\windows\apo\bin\x64\Release\voxveil-control.exe')
  )
  $ControlPath = $candidates | Where-Object { Test-Path $_ -PathType Leaf } | Select-Object -First 1
}

if (-not $ControlPath -or -not (Test-Path $ControlPath -PathType Leaf)) {
  throw 'voxveil-control.exe was not found. Build the Windows native control tools or pass -ControlPath.'
}

$statusOutput = & $ControlPath status 2>&1
if ($LASTEXITCODE -ne 0) {
  throw "voxveil-control status failed with exit code $LASTEXITCODE: $statusOutput"
}

$status = ($statusOutput -join ' ').Trim()
$values = @{}
foreach ($token in ($status -split '\s+')) {
  if ($token -match '^(?<name>[a-z-]+)=(?<value>-?[0-9]+)$') {
    $values[$Matches.name] = [int64]$Matches.value
  }
}

foreach ($required in @('heartbeat', 'loaded')) {
  if (-not $values.ContainsKey($required)) {
    throw "voxveil-control status did not contain required field '$required': $status"
  }
}

[pscustomobject]@{
  loaded = [uint32]$values.loaded
  capx = if ($values.ContainsKey('capx')) { [uint32]$values.capx } else { 0 }
  systemEffectEnabled = if ($values.ContainsKey('system-effect')) { [bool]$values.'system-effect' } else { $null }
  heartbeat = [uint32]$values.heartbeat
  processingReady = ([uint32]$values.loaded -gt 0)
  rawStatus = $status
} | ConvertTo-Json -Depth 2
