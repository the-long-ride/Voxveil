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

function Test-RecordedApoInfPresent([string]$PublishedInf) {
  $matches = @(Get-WindowsDriver -Online |
    Where-Object { [string]$_.Driver -ieq $PublishedInf })
  if ($matches.Count -eq 0) {
    return $false
  }
  if ($matches.Count -ne 1) {
    throw "Recorded APO/Extension package $PublishedInf resolved to $($matches.Count) Driver Store entries; install-state.json was kept for recovery."
  }

  $package = $matches[0]
  $originalName = [IO.Path]::GetFileName([string]$package.OriginalFileName)
  if ([string]$package.ProviderName -ine 'Voxveil' -or
      $originalName -notin @('VoxveilApo.inf', 'VoxveilApoExtension.inf')) {
    throw "Recorded package $PublishedInf no longer identifies a Voxveil APO/Extension package; install-state.json was kept for recovery."
  }
  return $true
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

function Remove-RecordedDevelopmentCertificate([string]$Thumbprint) {
  if (-not $Thumbprint) {
    return
  }
  if ($Thumbprint -notmatch '^[0-9A-Fa-f]{40}\z') {
    throw 'Recorded developmentCertificateThumbprint is invalid; install-state.json was kept for recovery.'
  }

  $certificatePaths = @(
    "Cert:\LocalMachine\My\$Thumbprint",
    "Cert:\LocalMachine\Root\$Thumbprint",
    "Cert:\LocalMachine\TrustedPublisher\$Thumbprint"
  )
  $existingPaths = [Collections.Generic.List[string]]::new()
  foreach ($certificatePath in $certificatePaths) {
    if (-not (Test-Path $certificatePath -PathType Leaf)) {
      continue
    }
    $certificate = Get-Item $certificatePath
    if ([string]$certificate.Subject -notmatch '(^|,\s*)CN=Voxveil Development APO($|,)') {
      throw "Recorded development certificate $Thumbprint resolves to an unexpected subject in $certificatePath; install-state.json was kept for recovery."
    }
    $existingPaths.Add($certificatePath)
  }

  foreach ($certificatePath in $existingPaths) {
    Remove-Item $certificatePath -Force
  }
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
$developmentCertificateThumbprint = $null
$legacyCertificateOwnershipUnknown = $false

if (Test-Path $statePath) {
  $state = Get-Content $statePath -Raw | ConvertFrom-Json
  $recordedInfNames = @($state.installedInfNames)
  $invalidRecordedInfNames = @($recordedInfNames | Where-Object { [string]$_ -notmatch '^oem\d+[.]inf\z' })
  if ($invalidRecordedInfNames.Count -gt 0) {
    throw "Malformed APO/Extension package identity in install-state.json: $($invalidRecordedInfNames -join ', '). State was kept for recovery."
  }
  $infNames = @($recordedInfNames)
  $bindingMode = [string]$state.bindingMode
  if ($bindingMode -notin @('capx-extension', 'legacy-runtime-interface', 'legacy-reference')) {
    throw "install-state.json has unknown bindingMode '$bindingMode'; state was kept for recovery."
  }
  $legacyRuntimeAttachedProperty = $state.PSObject.Properties['legacyRuntimeAttached']
  $legacyRuntimeAttached = if ($bindingMode -eq 'legacy-runtime-interface') {
    if ($legacyRuntimeAttachedProperty) { [bool]$legacyRuntimeAttachedProperty.Value } else { $true }
  } else {
    $false
  }
  $developmentCertificateThumbprint = [string]$state.developmentCertificateThumbprint
  if ($developmentCertificateThumbprint -and $developmentCertificateThumbprint -notmatch '^[0-9A-Fa-f]{40}\z') {
    throw 'install-state.json contains an invalid developmentCertificateThumbprint; state was kept for recovery.'
  }
  $legacyCertificateOwnershipUnknown = (
    $bindingMode -in @('legacy-runtime-interface', 'legacy-reference') -and
    -not $developmentCertificateThumbprint
  )
  if ($legacyCertificateOwnershipUnknown) {
    Write-Warning 'Existing legacy TestSign certificate ownership is unknown. This uninstall will remove only recorded APO/Extension state; review old Voxveil Development APO certificates manually instead of deleting trust-store certificates by subject.'
  }
  $pendingProperty = $state.PSObject.Properties['pendingReboot']
  $bootMarkerProperty = $state.PSObject.Properties['pendingRebootBootMarker']
  $pendingRemovedProperty = $state.PSObject.Properties['pendingRemovedInfName']
  $audioRestartProperty = $state.PSObject.Properties['audioServiceRestartRequired']
  $pendingReboot = $pendingProperty -and [bool]$pendingProperty.Value
  $pendingBootMarker = if ($bootMarkerProperty) { [string]$bootMarkerProperty.Value } else { '' }
  $pendingRemovedInfName = if ($pendingRemovedProperty) { [string]$pendingRemovedProperty.Value } else { '' }
  $audioServiceRestartRequired = $audioRestartProperty -and [bool]$audioRestartProperty.Value
  if ($pendingReboot -and -not $pendingBootMarker) {
    if ($bootMarkerProperty) {
      $state.pendingRebootBootMarker = $currentBootMarker
    } else {
      $state | Add-Member -NotePropertyName pendingRebootBootMarker -NotePropertyValue $currentBootMarker
    }
    Write-JsonStateAtomically -State $state -Path $statePath
    throw 'Restart Windows before continuing Voxveil APO package cleanup.'
  }
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
  if (-not $audioRestartProperty) {
    $state | Add-Member -NotePropertyName audioServiceRestartRequired -NotePropertyValue $false
  }
  if ($pendingRemovedInfName) {
    Assert-PendingRemovedApoInfAbsent $pendingRemovedInfName
    $state.pendingRemovedInfName = $null
    $state.pendingReboot = $false
    $state.pendingRebootBootMarker = $null
    Write-JsonStateAtomically -State $state -Path $statePath
  }
}

if ($state -and [string]$state.bindingMode -eq 'legacy-runtime-interface' -and $legacyRuntimeAttached) {
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
  if ($state.PSObject.Properties['legacyRuntimeAttached']) {
    $state.legacyRuntimeAttached = $false
  } else {
    $state | Add-Member -NotePropertyName legacyRuntimeAttached -NotePropertyValue $false
  }
  $legacyRuntimeAttached = $false
  Write-JsonStateAtomically -State $state -Path $statePath
}

if ($infNames.Count -eq 0) {
  if ($audioServiceRestartRequired) {
    Restart-Service Audiosrv -Force
    $state.audioServiceRestartRequired = $false
    Write-JsonStateAtomically -State $state -Path $statePath
  }
  Remove-RecordedDevelopmentCertificate $developmentCertificateThumbprint
  Remove-Item $statePath -Force -ErrorAction SilentlyContinue
  Write-Host 'No APO/Extension package identities were recorded; no driver packages were removed to avoid deleting other Voxveil components.'
  return
}

foreach ($inf in @($infNames)) {
  $packagePresentBeforeDelete = Test-RecordedApoInfPresent $inf
  if (-not $packagePresentBeforeDelete) {
    $infNames = @($infNames | Where-Object { $_ -ine $inf })
    $state.installedInfNames = @($infNames)
    $state.pendingRemovedInfName = $inf
    $state.pendingReboot = $true
    $state.pendingRebootBootMarker = $currentBootMarker
    $state.audioServiceRestartRequired = $false
    Write-JsonStateAtomically -State $state -Path $statePath
    Write-Warning "Recorded Voxveil APO/Extension package $inf is already absent. Treating this as an interrupted prior deletion and requiring a restart before cleanup continues."
    exit 3010
  }

  Assert-RecordedApoInfIdentity $inf
  Write-Host "Removing recorded Voxveil APO/Extension driver package $inf ..."
  pnputil.exe /delete-driver $inf /uninstall /force | Out-Host
  $pnputilExitCode = $LASTEXITCODE
  $packageStillPresent = Test-RecordedApoInfPresent $inf
  if ($pnputilExitCode -ne 0 -and $pnputilExitCode -ne 3010) {
    if (-not $packageStillPresent) {
      $infNames = @($infNames | Where-Object { $_ -ine $inf })
      $state.installedInfNames = @($infNames)
      $state.pendingRemovedInfName = $null
      $state.pendingReboot = $false
      $state.pendingRebootBootMarker = $null
      $state.audioServiceRestartRequired = ($infNames.Count -eq 0)
      Write-JsonStateAtomically -State $state -Path $statePath
    }
    throw "PnPUtil failed to remove $inf (exit $pnputilExitCode). Driver Store ownership was refreshed and install-state.json was kept for recovery."
  }
  if ($pnputilExitCode -eq 0 -and $packageStillPresent) {
    throw "PnPUtil reported success removing $inf, but the recorded Voxveil package is still present in the Driver Store; install-state.json was kept for recovery."
  }

  $infNames = @($infNames | Where-Object { $_ -ine $inf })
  $state.installedInfNames = @($infNames)
  if ($pnputilExitCode -eq 3010) {
    $state.pendingRemovedInfName = $inf
    $state.pendingReboot = $true
    $state.pendingRebootBootMarker = $currentBootMarker
    $state.audioServiceRestartRequired = $false
  } else {
    $state.pendingRemovedInfName = $null
    $state.pendingReboot = $false
    $state.pendingRebootBootMarker = $null
    $state.audioServiceRestartRequired = ($infNames.Count -eq 0)
  }
  Write-JsonStateAtomically -State $state -Path $statePath

  if ($pnputilExitCode -eq 3010) {
    Write-Warning "Voxveil APO/Extension package $inf was removed successfully, but Windows requires a restart before remaining package cleanup can continue."
    exit 3010
  }
}

if ($state.audioServiceRestartRequired) {
  Restart-Service Audiosrv -Force
  $state.audioServiceRestartRequired = $false
  Write-JsonStateAtomically -State $state -Path $statePath
}
Remove-RecordedDevelopmentCertificate $developmentCertificateThumbprint
Remove-Item $statePath -Force -ErrorAction SilentlyContinue
Write-Host 'Recorded Voxveil componentized APO packages removed.'
