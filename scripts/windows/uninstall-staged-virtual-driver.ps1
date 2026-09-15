[CmdletBinding()]
param(
  [string]$PackageDir
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-Administrator {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($identity)
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this script from an elevated PowerShell (Run as administrator).'
  }
}

function Assert-PublishedInfIdentity([string]$PublishedInf, [string]$DeviceInstanceId) {
  $boundDrivers = @(Get-CimInstance Win32_PnPSignedDriver |
    Where-Object { $_.InfName -ieq $PublishedInf })

  if ($boundDrivers.Count -gt 0) {
    if ($boundDrivers.Count -ne 1) {
      throw "Recorded package $PublishedInf is bound to $($boundDrivers.Count) devices. Refusing broad deletion; the install-state file was kept for recovery."
    }
    $binding = $boundDrivers[0]
    if ($binding.DriverProviderName -ine 'Voxveil' -or
        $binding.DeviceName -ine 'Voxveil Virtual Audio' -or
        $binding.DeviceID -ine $DeviceInstanceId) {
      throw "Recorded package $PublishedInf no longer belongs exclusively to the recorded Voxveil devnode. Refusing deletion; the install-state file was kept for recovery."
    }
    return
  }

  # The devnode may already have been removed by a previous interrupted uninstall.
  # In that recovery case, verify the current published INF file itself before
  # deleting an unbound driver-store package. This also rejects oemN.inf reuse.
  $publishedInfPath = Join-Path $env:windir "INF\$PublishedInf"
  if (-not (Test-Path $publishedInfPath -PathType Leaf)) {
    throw "Recorded package $PublishedInf is neither bound nor present under Windows\INF. The install-state file was kept for recovery."
  }
  $publishedText = Get-Content $publishedInfPath -Raw
  if ($publishedText -notmatch '(?im)Root\\VoxveilVirtualAudio' -or
      $publishedText -notmatch '(?im)^\s*CatalogFile\s*=\s*VoxveilVirtualAudio\.cat\s*$' -or
      $publishedText -notmatch '(?im)^\s*ProviderName\s*=\s*"Voxveil"\s*$' -or
      $publishedText -notmatch '(?im)^\s*DeviceName\s*=\s*"Voxveil Virtual Audio"\s*$') {
    throw "Recorded package $PublishedInf is no longer identifiable as Voxveil Virtual Audio. Refusing deletion; the install-state file was kept for recovery."
  }
}

Assert-Administrator

if (-not $PackageDir) {
  $PackageDir = Join-Path $PSScriptRoot 'virtual-driver'
}
$package = [IO.Path]::GetFullPath($PackageDir)
$statePath = Join-Path $package 'virtual-driver-install-state.json'
if (-not (Test-Path $statePath -PathType Leaf)) {
  Write-Host 'No recorded Voxveil virtual-driver package identity was found; no driver package was removed.'
  return
}

$state = Get-Content $statePath -Raw | ConvertFrom-Json
$publishedInf = [string]$state.publishedInf
$deviceInstanceId = [string]$state.deviceInstanceId
if ($publishedInf -notmatch '^oem\d+\.inf$') {
  throw 'virtual-driver-install-state.json does not contain a valid publishedInf value.'
}
if (-not $deviceInstanceId -or $deviceInstanceId -match '[\r\n]') {
  throw 'virtual-driver-install-state.json does not contain a valid deviceInstanceId value.'
}

$deviceHelper = Join-Path $PSScriptRoot 'voxveil-virtual-device.exe'
if (-not (Test-Path $deviceHelper -PathType Leaf)) {
  throw "Voxveil root-device helper is missing: $deviceHelper"
}

Assert-PublishedInfIdentity $publishedInf $deviceInstanceId

Write-Host "Removing recorded Voxveil Virtual Audio devnode $deviceInstanceId ..."
& $deviceHelper remove $deviceInstanceId | Out-Host
if ($LASTEXITCODE -ne 0) {
  throw "voxveil-virtual-device.exe failed to remove the recorded devnode (exit $LASTEXITCODE). The install-state file was kept for recovery."
}

Write-Host "Deleting recorded Voxveil Virtual Audio driver-store package $publishedInf ..."
pnputil.exe /delete-driver $publishedInf | Out-Host
if ($LASTEXITCODE -ne 0) {
  throw "PnPUtil failed to delete $publishedInf (exit $LASTEXITCODE). The install-state file was kept for recovery."
}

Remove-Item $statePath -Force
Write-Host 'Recorded Voxveil Virtual Audio devnode and driver package removed.'
