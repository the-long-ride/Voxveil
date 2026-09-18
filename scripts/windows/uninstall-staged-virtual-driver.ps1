[CmdletBinding()]
param(
  [string]$PackageDir
)

$ErrorActionPreference = 'Stop'

$trustedSystemDirectoryForModules = [Environment]::SystemDirectory
if (-not $trustedSystemDirectoryForModules) {
  throw 'Windows system directory could not be resolved for PowerShell module loading.'
}
$trustedModulePath = Join-Path $trustedSystemDirectoryForModules 'WindowsPowerShell\v1.0\Modules'
if (-not (Test-Path $trustedModulePath -PathType Container)) {
  throw "Trusted Windows PowerShell module directory was not found: $trustedModulePath"
}
$env:PSModulePath = $trustedModulePath
$trustedWindowsDirectory = Split-Path -Parent $trustedSystemDirectoryForModules
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
  $publishedInfPath = Join-Path $trustedWindowsDirectory "INF\$PublishedInf"
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

function Test-RecordedVirtualDriverPackagePresent([string]$PublishedInf) {
  $matches = @(Get-WindowsDriver -Online |
    Where-Object { [string]$_.Driver -ieq $PublishedInf })
  if ($matches.Count -eq 0) {
    return $false
  }
  if ($matches.Count -ne 1) {
    throw "Recorded package $PublishedInf resolved to $($matches.Count) Driver Store entries; the install-state file was kept for recovery."
  }

  $package = $matches[0]
  if ([string]$package.ProviderName -ine 'Voxveil' -or
      [IO.Path]::GetFileName([string]$package.OriginalFileName) -ine 'VoxveilVirtualAudio.inf') {
    throw "Recorded package $PublishedInf no longer identifies Voxveil Virtual Audio; the install-state file was kept for recovery."
  }
  return $true
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
  if ($publishedInf -notmatch '^oem\d+\.inf$' -or -not $deviceInstanceId -or $deviceInstanceId -match '[\r\n]') {
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
    throw 'Voxveil root-device helper returned invalid query metadata; uninstall state was kept for recovery.'
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

function Write-JsonStateAtomically($State, [string]$Path) {
  $directory = Split-Path -Parent $Path
  $tempPath = Join-Path $directory ('.' + [IO.Path]::GetFileName($Path) + '.' + [Guid]::NewGuid().ToString('N') + '.tmp')
  try {
    $State | ConvertTo-Json -Depth 3 | Set-Content $tempPath -Encoding utf8
    if (Test-Path $Path -PathType Leaf) {
      [IO.File]::Replace($tempPath, $Path, $null)
    } else {
      [IO.File]::Move($tempPath, $Path)
    }
  }
  finally {
    Remove-Item $tempPath -Force -ErrorAction SilentlyContinue
  }
}

function Get-HelperValue([string[]]$Output, [string]$Name) {
  $prefix = "$Name="
  $line = $Output | Where-Object { $_.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) } | Select-Object -First 1
  if (-not $line) { return $null }
  return $line.Substring($prefix.Length).Trim()
}

Assert-Administrator

$trustedSystemDirectory = [Environment]::SystemDirectory
if (-not $trustedSystemDirectory) {
  throw 'Windows system directory could not be resolved for PnPUtil.'
}
$pnputilPath = Join-Path $trustedSystemDirectory 'pnputil.exe'
if (-not (Test-Path $pnputilPath -PathType Leaf)) {
  throw "PnPUtil was not found at $pnputilPath."
}
Set-Alias -Name 'pnputil.exe' -Value $pnputilPath -Scope Script -Option ReadOnly

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

if ($pendingReboot -and -not $pendingBootMarker) {
  if ($bootMarkerProperty) {
    $state.pendingRebootBootMarker = $currentBootMarker
  } else {
    $state | Add-Member -NotePropertyName pendingRebootBootMarker -NotePropertyValue $currentBootMarker
  }
  Write-JsonStateAtomically -State $state -Path $statePath
  throw 'Restart Windows before continuing Voxveil Virtual Audio lifecycle changes.'
}
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

$packagePresentBeforeDelete = Test-RecordedVirtualDriverPackagePresent $publishedInf
if (-not $packagePresentBeforeDelete) {
  $queryOutput = @(& $deviceHelper query $deviceInstanceId)
  $queryExitCode = $LASTEXITCODE
  if ($queryExitCode -ne 0) {
    throw "Recorded package $publishedInf is already absent, but the Voxveil devnode could not be queried safely; install state was kept for recovery."
  }
  $queryExists = Get-HelperValue $queryOutput 'exists'
  if ($queryExists -notin @('0', '1')) {
    throw 'Voxveil root-device helper returned invalid query metadata while recovering an interrupted uninstall.'
  }
  if ($queryExists -eq '1') {
    $recoveryRemoveOutput = @(& $deviceHelper remove $deviceInstanceId)
    $recoveryRemoveExitCode = $LASTEXITCODE
    $recoveryRemoveOutput | Out-Host
    if ($recoveryRemoveExitCode -ne 0) {
      throw "Recorded package $publishedInf is already absent, but removing the exact Voxveil devnode failed (exit $recoveryRemoveExitCode); install state was kept for recovery."
    }
    $recoveryRebootValue = Get-HelperValue $recoveryRemoveOutput 'rebootRequired'
    if ($recoveryRebootValue -notin @('0', '1')) {
      throw 'Voxveil root-device helper returned invalid reboot metadata while recovering an interrupted uninstall.'
    }
  }

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
  Write-JsonStateAtomically -State $state -Path $statePath
  Write-Warning "Recorded Voxveil package $publishedInf is already absent. Treating this as an interrupted prior deletion and requiring a restart before lifecycle state is cleared."
  exit 3010
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
$packageStillPresent = Test-RecordedVirtualDriverPackagePresent $publishedInf
if ($pnputilExitCode -ne 0 -and $pnputilExitCode -ne 3010) {
  if (-not $packageStillPresent) {
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
    Write-JsonStateAtomically -State $state -Path $statePath
    Write-Warning "PnPUtil returned exit $pnputilExitCode for $publishedInf, but the package is already absent. Requiring a restart before lifecycle state is cleared because completion is ambiguous."
    exit 3010
  } elseif ($helperRebootRequired) {
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
    Write-JsonStateAtomically -State $state -Path $statePath
  }
  throw "PnPUtil failed to delete $publishedInf (exit $pnputilExitCode). Driver Store ownership was refreshed before preserving recovery state."
}
if ($pnputilExitCode -eq 0 -and $packageStillPresent) {
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
    Write-JsonStateAtomically -State $state -Path $statePath
  }
  throw "PnPUtil reported success deleting $publishedInf, but the recorded Voxveil package is still present in the Driver Store; the install-state file was kept for recovery."
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
  Write-JsonStateAtomically -State $state -Path $statePath
  Write-Warning 'Voxveil Virtual Audio was removed successfully, but Windows requires a restart to finish the device/package removal.'
  exit 3010
}

Remove-Item $statePath -Force
Write-Host 'Recorded Voxveil Virtual Audio devnode and driver package removed.'