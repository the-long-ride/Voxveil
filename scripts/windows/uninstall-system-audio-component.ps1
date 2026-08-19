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
$infNames = @()

if (Test-Path $statePath) {
  $state = Get-Content $statePath -Raw | ConvertFrom-Json
  $infNames = @($state.installedInfNames) | Where-Object { $_ -match '^oem\d+\.inf$' }
}

if ($infNames.Count -eq 0) {
  $infNames = @(Get-CimInstance Win32_PnPSignedDriver |
    Where-Object { $_.DriverProviderName -eq 'Voxveil' -and $_.InfName -match '^oem\d+\.inf$' } |
    Select-Object -ExpandProperty InfName -Unique)
}

if ($infNames.Count -eq 0) {
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
