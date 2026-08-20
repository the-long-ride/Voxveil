[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw 'Run this script from an elevated PowerShell (Run as administrator).'
}

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$statePath = Join-Path $root 'install-state.json'
$control = Join-Path $root 'voxveil-control.exe'
$infNames = @()
$state = $null

if (Test-Path $statePath) {
  $state = Get-Content $statePath -Raw | ConvertFrom-Json
  $infNames = @($state.installedInfNames) | Where-Object { $_ -match '^oem\d+\.inf$' }
}

if ($state -and [string]$state.bindingMode -eq 'runtime-interface') {
  foreach ($name in @('bindingPnpInstanceId', 'topologyInterfacePath', 'audioInterfacePath')) {
    if (-not $state.$name) {
      throw "Cannot safely detach Voxveil runtime FX registration: install-state.json is missing $name."
    }
  }
  if (-not (Test-Path $control -PathType Leaf)) {
    throw 'Cannot safely detach Voxveil runtime FX registration: voxveil-control.exe is missing.'
  }
  Write-Host 'Detaching Voxveil FX properties from the selected Windows audio interfaces...'
  & $control detach-effects `
    ([string]$state.bindingPnpInstanceId) `
    ([string]$state.topologyInterfacePath) `
    ([string]$state.audioInterfacePath) | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "Runtime interface FX detach failed (exit $LASTEXITCODE); driver packages were left installed to avoid a stale APO registration."
  }
}

if ($infNames.Count -eq 0) {
  $infNames = @(Get-CimInstance Win32_PnPSignedDriver |
    Where-Object { $_.DriverProviderName -eq 'Voxveil' -and $_.InfName -match '^oem\d+\.inf$' } |
    Select-Object -ExpandProperty InfName -Unique)
}

if ($infNames.Count -eq 0) {
  Remove-Item $statePath -Force -ErrorAction SilentlyContinue
  Write-Host 'No installed Voxveil driver packages were found.'
  return
}

foreach ($inf in $infNames) {
  Write-Host "Removing Voxveil driver package $inf ..."
  pnputil.exe /delete-driver $inf /uninstall /force | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "PnPUtil failed to remove $inf (exit $LASTEXITCODE)."
  }
}

Remove-Item $statePath -Force -ErrorAction SilentlyContinue
Restart-Service Audiosrv -Force
Write-Host 'Voxveil componentized APO packages removed.'
