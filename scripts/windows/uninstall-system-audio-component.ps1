[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

function Assert-RecordedApoInfIdentity([string]$PublishedInf) {
  $matches = @(Get-WindowsDriver -Online |
    Where-Object { [string]$_.Driver -ieq $PublishedInf })
  if ($matches.Count -ne 1) {
    throw "Recorded APO/Extension package $PublishedInf is not present exactly once in the Driver Store. Refusing deletion; install-state.json was kept for recovery."
  }

  $package = $matches[0]
  $originalName = [IO.Path]::GetFileName([string]$package.OriginalFileName)
  if ([string]$package.ProviderName -ine 'Voxveil' -or
      $originalName -notin @('VoxveilApo.inf', 'VoxveilApoExtension.inf')) {
    throw "Recorded package $PublishedInf no longer identifies a Voxveil APO/Extension package. Refusing deletion; install-state.json was kept for recovery."
  }
}

function Assert-PendingRemovedApoInfAbsent([string]$PublishedInf) {
  if ($PublishedInf -notmatch '^oem\d+\.inf$') {
    throw 'Pending removed APO/Extension package identity is invalid; install-state.json was kept for recovery.'
  }

  $matches = @(Get-WindowsDriver -Online |
    Where-Object {
      [string]$_.Driver -ieq $PublishedInf -and
      [string]$_.ProviderName -ieq 'Voxveil' -and
      [IO.Path]::GetFileName([string]$_.OriginalFileName) -in @('VoxveilApo.inf', 'VoxveilApoExtension.inf')
    })
  if ($matches.Count -gt 0) {
    throw "Recorded APO/Extension package $PublishedInf still exists after the required restart; install-state.json was kept for recovery."
  }
}

function Get-WindowsBootMarker {
  $os = Get-CimInstance Win32_OperatingSystem
  if (-not $os.LastBootUpTime) {
    throw 'Could not determine the current Windows boot marker.'
  }
  ([DateTime]$os.LastBootUpTime).ToUniversalTime().ToString('o')
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw 'Run this script from an elevated PowerShell (Run as administrator).'
}

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$statePath = Join-Path $root 'install-state.json'
$control = Join-Path $root 'voxveil-control.exe'
$currentBootMarker = Get-WindowsBootMarker
$infNames = @()
$state = $null

if (Test-Path $statePath) {
  $state = Get-Content $statePath -Raw | ConvertFrom-Json
  $infNames = @($state.installedInfNames) | Where-Object { $_ -match '^oem\d+\.inf$' }
  $pendingProperty = $state.PSObject.Properties['pendingReboot']
  $bootMarkerProperty = $state.PSObject.Properties['pendingRebootBootMarker']
  $pendingRemovedProperty = $state.PSObject.Properties['pendingRemovedInfName']
  $pendingReboot = $pendingProperty -and [bool]$pendingProperty.Value
  $pendingBootMarker = if ($bootMarkerProperty) { [string]$bootMarkerProperty.Value } else { '' }
  $pendingRemovedInfName = if ($pendingRemovedProperty) { [string]$pendingRemovedProperty.Value } else { '' }
  if ($pendingReboot -and $pendingBootMarker -and $pendingBootMarker -eq $currentBootMarker) {
    throw 'Restart Windows before continuing Voxveil APO package cleanup.'
  }
  if (-not $pendingProperty) {
    $state | Add-Member -NotePropertyName pendingReboot -NotePropertyValue $false
  }
  if (-not $bootMarkerProperty) {
    $state | Add-Member -NotePropertyName pendingRebootBootMarker -NotePropertyValue $null
  }
  if (-not $pendingRemovedProperty) {
    $state | Add-Member -NotePropertyName pendingRemovedInfName -NotePropertyValue $null
  }
  if ($pendingRemovedInfName) {
    Assert-PendingRemovedApoInfAbsent $pendingRemovedInfName
    $state.pendingRemovedInfName = $null
    $state.pendingReboot = $false
    $state.pendingRebootBootMarker = $null
    $state | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8
  }
}

if ($state -and [string]$state.bindingMode -eq 'legacy-runtime-interface') {
  foreach ($name in @('bindingPnpInstanceId', 'topologyInterfacePath', 'audioInterfacePath')) {
    if (-not $state.$name) {
      throw "Cannot safely detach Voxveil runtime FX registration: install-state.json is missing $name."
    }
  }
  if (-not (Test-Path $control -PathType Leaf)) {
    throw 'Cannot safely detach Voxveil runtime FX registration: voxveil-control.exe is missing.'
  }
  Write-Host 'Detaching Voxveil legacy development FX properties from the selected Windows audio interfaces...'
  & $control detach-effects `
    ([string]$state.bindingPnpInstanceId) `
    ([string]$state.topologyInterfacePath) `
    ([string]$state.audioInterfacePath) | Out-Host
  if ($LASTEXITCODE -ne 0) {
    throw "Runtime interface FX detach failed (exit $LASTEXITCODE); driver packages were left installed to avoid a stale APO registration."
  }
}

if ($infNames.Count -eq 0) {
  Remove-Item $statePath -Force -ErrorAction SilentlyContinue
  Write-Host 'No APO/Extension package identities were recorded; no driver packages were removed to avoid deleting other Voxveil components.'
  return
}

foreach ($inf in @($infNames)) {
  Assert-RecordedApoInfIdentity $inf
  Write-Host "Removing recorded Voxveil APO/Extension driver package $inf ..."
  pnputil.exe /delete-driver $inf /uninstall /force | Out-Host
  $pnputilExitCode = $LASTEXITCODE
  if ($pnputilExitCode -ne 0 -and $pnputilExitCode -ne 3010) {
    throw "PnPUtil failed to remove $inf (exit $pnputilExitCode)."
  }

  $infNames = @($infNames | Where-Object { $_ -ine $inf })
  $state.installedInfNames = @($infNames)
  if ($pnputilExitCode -eq 3010) {
    $state.pendingRemovedInfName = $inf
    $state.pendingReboot = $true
    $state.pendingRebootBootMarker = $currentBootMarker
  } else {
    $state.pendingRemovedInfName = $null
    $state.pendingReboot = $false
    $state.pendingRebootBootMarker = $null
  }
  $state | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8

  if ($pnputilExitCode -eq 3010) {
    Write-Warning "Voxveil APO/Extension package $inf was removed successfully, but Windows requires a restart before remaining package cleanup can continue."
    exit 3010
  }
}

Remove-Item $statePath -Force -ErrorAction SilentlyContinue
Restart-Service Audiosrv -Force
Write-Host 'Recorded Voxveil componentized APO packages removed.'
