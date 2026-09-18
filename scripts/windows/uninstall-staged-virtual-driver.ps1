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

function Assert-CompletedUninstallAbsent($State) {
  $publishedInfProperty = $State.PSObject.Properties['publishedInf']
  $deviceInstanceProperty = $State.PSObject.Properties['deviceInstanceId']

  # Package-less rollback tombstones record only the restart boundary.
  if (-not $publishedInfProperty -and -not $deviceInstanceProperty) {
    return
  }
  if (-not $publishedInfProperty -or -not $deviceInstanceProperty) {
    throw 'Completed virtual-driver uninstall tombstone has incomplete ownership identity; state was kept for recovery.'
  }

  $publishedInf = [string]$publishedInfProperty.Value
  $deviceInstanceId = [string]$deviceInstanceProperty.Value
  if ($publishedInf -notmatch '^oem\d+\.inf
  $os = Get-CimInstance Win32_OperatingSystem
  if (-not $os.LastBootUpTime) {
    throw 'Could not determine the current Windows boot marker.'
  }
  ([DateTime]$os.LastBootUpTime).ToUniversalTime().ToString('o')
}

function Get-HelperValue([string[]]$Output, [string]$Name) {
  $prefix = "$Name="
  $line = $Output | Where-Object { $_.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) } | Select-Object -First 1
  if (-not $line) { return $null }
  return $line.Substring($prefix.Length).Trim()
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
$currentBootMarker = Get-WindowsBootMarker
$uninstallCompleteProperty = $state.PSObject.Properties['uninstallComplete']
$pendingProperty = $state.PSObject.Properties['pendingReboot']
$bootMarkerProperty = $state.PSObject.Properties['pendingRebootBootMarker']
$uninstallComplete = $uninstallCompleteProperty -and [bool]$uninstallCompleteProperty.Value
$pendingReboot = $pendingProperty -and [bool]$pendingProperty.Value
$pendingBootMarker = if ($bootMarkerProperty) { [string]$bootMarkerProperty.Value } else { '' }

if ($pendingReboot -and $pendingBootMarker -and $pendingBootMarker -eq $currentBootMarker) {
  throw 'Restart Windows before continuing Voxveil Virtual Audio lifecycle changes.'
}

if ($uninstallComplete) {
  Assert-CompletedUninstallAbsent $state
  Remove-Item $statePath -Force
  Write-Host 'Prior Voxveil Virtual Audio uninstall completed after restart; reboot tombstone removed.'
  return
}

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
$removeOutput = @(& $deviceHelper remove $deviceInstanceId)
$removeExitCode = $LASTEXITCODE
$removeOutput | Out-Host
if ($removeExitCode -ne 0) {
  throw "voxveil-virtual-device.exe failed to remove the recorded devnode (exit $removeExitCode). The install-state file was kept for recovery."
}
$removeRebootValue = Get-HelperValue $removeOutput 'rebootRequired'
if ($removeRebootValue -notin @('0', '1')) {
  throw 'voxveil-virtual-device.exe returned invalid remove reboot metadata. The install-state file was kept for recovery.'
}
$helperRebootRequired = $removeRebootValue -eq '1'

Write-Host "Deleting recorded Voxveil Virtual Audio driver-store package $publishedInf ..."
pnputil.exe /delete-driver $publishedInf | Out-Host
$pnputilExitCode = $LASTEXITCODE
if ($pnputilExitCode -ne 0 -and $pnputilExitCode -ne 3010) {
  if ($helperRebootRequired) {
    if ($state.PSObject.Properties['pendingReboot']) {
      $state.pendingReboot = $true
    } else {
      $state | Add-Member -NotePropertyName pendingReboot -NotePropertyValue $true
    }
    if ($state.PSObject.Properties['pendingRebootBootMarker']) {
      $state.pendingRebootBootMarker = $currentBootMarker
    } else {
      $state | Add-Member -NotePropertyName pendingRebootBootMarker -NotePropertyValue $currentBootMarker
    }
    if ($state.PSObject.Properties['uninstallComplete']) {
      $state.uninstallComplete = $false
    } else {
      $state | Add-Member -NotePropertyName uninstallComplete -NotePropertyValue $false
    }
    $state | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8
  }
  throw "PnPUtil failed to delete $publishedInf (exit $pnputilExitCode). The install-state file was kept for recovery."
}

$lifecycleRebootRequired = $helperRebootRequired -or $pnputilExitCode -eq 3010
if ($lifecycleRebootRequired) {
  if ($state.PSObject.Properties['pendingReboot']) {
    $state.pendingReboot = $true
  } else {
    $state | Add-Member -NotePropertyName pendingReboot -NotePropertyValue $true
  }
  if ($state.PSObject.Properties['pendingRebootBootMarker']) {
    $state.pendingRebootBootMarker = $currentBootMarker
  } else {
    $state | Add-Member -NotePropertyName pendingRebootBootMarker -NotePropertyValue $currentBootMarker
  }
  if ($state.PSObject.Properties['uninstallComplete']) {
    $state.uninstallComplete = $true
  } else {
    $state | Add-Member -NotePropertyName uninstallComplete -NotePropertyValue $true
  }
  $state | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8
  Write-Warning 'Voxveil Virtual Audio was removed successfully, but Windows requires a restart to finish the device/package removal.'
  exit 3010
}

Remove-Item $statePath -Force
Write-Host 'Recorded Voxveil Virtual Audio devnode and driver package removed.' -or -not $deviceInstanceId -or $deviceInstanceId -match '[\r\n]') {
    throw 'Completed virtual-driver uninstall tombstone has invalid ownership identity; state was kept for recovery.'
  }

  $remainingPackages = @(Get-WindowsDriver -Online |
    Where-Object {
      [string]$_.Driver -ieq $publishedInf -and
      $_.ProviderName -ieq 'Voxveil' -and
      [IO.Path]::GetFileName([string]$_.OriginalFileName) -ieq 'VoxveilVirtualAudio.inf'
    })
  if ($remainingPackages.Count -gt 0) {
    throw "Recorded Voxveil package $publishedInf still exists after the required restart; uninstall state was kept for recovery."
  }

  $queryHelper = Join-Path $PSScriptRoot 'voxveil-virtual-device.exe'
  if (-not (Test-Path $queryHelper -PathType Leaf)) {
    throw "Voxveil root-device helper is missing: $queryHelper"
  }
  $queryOutput = @(& $queryHelper query $deviceInstanceId)
  $queryExitCode = $LASTEXITCODE
  if ($queryExitCode -ne 0) {
    throw "Could not verify whether recorded Voxveil devnode $deviceInstanceId is absent after restart; uninstall state was kept for recovery."
  }
  $queryExists = Get-HelperValue $queryOutput 'exists'
  if ($queryExists -notin @('0', '1')) {
    throw "Voxveil root-device helper returned invalid query metadata; uninstall state was kept for recovery."
  }
  if ($queryExists -eq '1') {
    throw "Recorded Voxveil devnode $deviceInstanceId still exists after the required restart; uninstall state was kept for recovery."
  }
}

function Get-WindowsBootMarker {
  $os = Get-CimInstance Win32_OperatingSystem
  if (-not $os.LastBootUpTime) {
    throw 'Could not determine the current Windows boot marker.'
  }
  ([DateTime]$os.LastBootUpTime).ToUniversalTime().ToString('o')
}

function Get-HelperValue([string[]]$Output, [string]$Name) {
  $prefix = "$Name="
  $line = $Output | Where-Object { $_.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) } | Select-Object -First 1
  if (-not $line) { return $null }
  return $line.Substring($prefix.Length).Trim()
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
$currentBootMarker = Get-WindowsBootMarker
$uninstallCompleteProperty = $state.PSObject.Properties['uninstallComplete']
$pendingProperty = $state.PSObject.Properties['pendingReboot']
$bootMarkerProperty = $state.PSObject.Properties['pendingRebootBootMarker']
$uninstallComplete = $uninstallCompleteProperty -and [bool]$uninstallCompleteProperty.Value
$pendingReboot = $pendingProperty -and [bool]$pendingProperty.Value
$pendingBootMarker = if ($bootMarkerProperty) { [string]$bootMarkerProperty.Value } else { '' }

if ($pendingReboot -and $pendingBootMarker -and $pendingBootMarker -eq $currentBootMarker) {
  throw 'Restart Windows before continuing Voxveil Virtual Audio lifecycle changes.'
}

if ($uninstallComplete) {
  Remove-Item $statePath -Force
  Write-Host 'Prior Voxveil Virtual Audio uninstall completed after restart; reboot tombstone removed.'
  return
}

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
$removeOutput = @(& $deviceHelper remove $deviceInstanceId)
$removeExitCode = $LASTEXITCODE
$removeOutput | Out-Host
if ($removeExitCode -ne 0) {
  throw "voxveil-virtual-device.exe failed to remove the recorded devnode (exit $removeExitCode). The install-state file was kept for recovery."
}
$removeRebootValue = Get-HelperValue $removeOutput 'rebootRequired'
if ($removeRebootValue -notin @('0', '1')) {
  throw 'voxveil-virtual-device.exe returned invalid remove reboot metadata. The install-state file was kept for recovery.'
}
$helperRebootRequired = $removeRebootValue -eq '1'

Write-Host "Deleting recorded Voxveil Virtual Audio driver-store package $publishedInf ..."
pnputil.exe /delete-driver $publishedInf | Out-Host
$pnputilExitCode = $LASTEXITCODE
if ($pnputilExitCode -ne 0 -and $pnputilExitCode -ne 3010) {
  if ($helperRebootRequired) {
    if ($state.PSObject.Properties['pendingReboot']) {
      $state.pendingReboot = $true
    } else {
      $state | Add-Member -NotePropertyName pendingReboot -NotePropertyValue $true
    }
    if ($state.PSObject.Properties['pendingRebootBootMarker']) {
      $state.pendingRebootBootMarker = $currentBootMarker
    } else {
      $state | Add-Member -NotePropertyName pendingRebootBootMarker -NotePropertyValue $currentBootMarker
    }
    if ($state.PSObject.Properties['uninstallComplete']) {
      $state.uninstallComplete = $false
    } else {
      $state | Add-Member -NotePropertyName uninstallComplete -NotePropertyValue $false
    }
    $state | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8
  }
  throw "PnPUtil failed to delete $publishedInf (exit $pnputilExitCode). The install-state file was kept for recovery."
}

$lifecycleRebootRequired = $helperRebootRequired -or $pnputilExitCode -eq 3010
if ($lifecycleRebootRequired) {
  if ($state.PSObject.Properties['pendingReboot']) {
    $state.pendingReboot = $true
  } else {
    $state | Add-Member -NotePropertyName pendingReboot -NotePropertyValue $true
  }
  if ($state.PSObject.Properties['pendingRebootBootMarker']) {
    $state.pendingRebootBootMarker = $currentBootMarker
  } else {
    $state | Add-Member -NotePropertyName pendingRebootBootMarker -NotePropertyValue $currentBootMarker
  }
  if ($state.PSObject.Properties['uninstallComplete']) {
    $state.uninstallComplete = $true
  } else {
    $state | Add-Member -NotePropertyName uninstallComplete -NotePropertyValue $true
  }
  $state | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8
  Write-Warning 'Voxveil Virtual Audio was removed successfully, but Windows requires a restart to finish the device/package removal.'
  exit 3010
}

Remove-Item $statePath -Force
Write-Host 'Recorded Voxveil Virtual Audio devnode and driver package removed.'