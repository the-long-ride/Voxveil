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

function Assert-SupportedWindowsBuild {
  $version = Get-ItemProperty `
    -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' `
    -Name CurrentBuildNumber `
    -ErrorAction Stop
  $build = 0
  if (-not [int]::TryParse([string]$version.CurrentBuildNumber, [ref]$build)) {
    throw "Unable to determine the current Windows build from CurrentBuildNumber='$($version.CurrentBuildNumber)'."
  }
  if ($build -lt 22621) {
    throw "Voxveil Virtual Audio requires Windows build 22621 or later; current build is $build. Use the Tier 1 relay path on supported older Windows releases."
  }
}

function Assert-StagedArchitecture {
  param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('x64', 'ARM64')]
    [string]$PackageArchitecture
  )

  $architectures = @(
    Get-CimInstance Win32_Processor -ErrorAction Stop |
      Select-Object -ExpandProperty Architecture -Unique
  )
  if ($architectures.Count -ne 1) {
    throw "Unable to determine one native processor architecture from Win32_Processor (found $($architectures.Count))."
  }

  $nativeArchitecture = switch ([int]$architectures[0]) {
    9 { 'x64'; break }
    12 { 'ARM64'; break }
    default { throw "Voxveil Virtual Audio does not support native processor architecture value $($architectures[0])." }
  }

  if ($PackageArchitecture -ine $nativeArchitecture) {
    throw "Verified virtual-driver architecture '$PackageArchitecture' does not match native Windows architecture '$nativeArchitecture'."
  }
}

function Assert-StagedFileHash(
  [Parameter(Mandatory = $true)][string]$Path,
  [Parameter(Mandatory = $true)][string]$ExpectedSha256,
  [Parameter(Mandatory = $true)][string]$Description
) {
  if (-not (Test-Path $Path -PathType Leaf)) {
    throw "Verified virtual-driver artifact is missing: $Description ($Path)"
  }
  if ($ExpectedSha256 -notmatch '^[0-9A-Fa-f]{64}$') {
    throw "verification.json contains an invalid SHA-256 value for $Description."
  }
  $actual = (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $ExpectedSha256.ToLowerInvariant()) {
    throw "Verified virtual-driver artifact changed after staging: $Description."
  }
}

function Get-HelperValue([string[]]$Output, [string]$Name) {
  $prefix = "$Name="
  $line = $Output | Where-Object { $_.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) } | Select-Object -First 1
  if (-not $line) { return $null }
  return $line.Substring($prefix.Length).Trim()
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
    throw "Recorded Voxveil package $publishedInf still exists after the required restart; install state was kept for recovery."
  }

  $queryHelper = Join-Path $PSScriptRoot 'voxveil-virtual-device.exe'
  if (-not (Test-Path $queryHelper -PathType Leaf)) {
    throw "Voxveil root-device helper is missing: $queryHelper"
  }
  $queryOutput = @(& $queryHelper query $deviceInstanceId)
  $queryExitCode = $LASTEXITCODE
  if ($queryExitCode -ne 0) {
    throw "Could not verify whether recorded Voxveil devnode $deviceInstanceId is absent after restart; install state was kept for recovery."
  }
  $queryExists = Get-HelperValue $queryOutput 'exists'
  if ($queryExists -notin @('0', '1')) {
    throw 'Voxveil root-device helper returned invalid query metadata; install state was kept for recovery.'
  }
  if ($queryExists -eq '1') {
    throw "Recorded Voxveil devnode $deviceInstanceId still exists after the required restart; install state was kept for recovery."
  }
}

function Get-WindowsBootMarker {
  $os = Get-CimInstance Win32_OperatingSystem
  if (-not $os.LastBootUpTime) {
    throw 'Could not determine the current Windows boot marker.'
  }
  ([DateTime]$os.LastBootUpTime).ToUniversalTime().ToString('o')
}

function Get-VoxveilPublishedInfNames {
  @(Get-WindowsDriver -Online |
    Where-Object {
      $_.ProviderName -ieq 'Voxveil' -and
      [IO.Path]::GetFileName([string]$_.OriginalFileName) -ieq 'VoxveilVirtualAudio.inf' -and
      [string]$_.Driver -match '^oem\d+\.inf$'
    } |
    ForEach-Object { [string]$_.Driver } |
    Sort-Object -Unique)
}

Assert-Administrator
Assert-SupportedWindowsBuild

if (-not $PackageDir) {
  $PackageDir = Join-Path $PSScriptRoot 'virtual-driver'
}
$package = [IO.Path]::GetFullPath($PackageDir)
if (-not (Test-Path $package -PathType Container)) {
  throw "Staged virtual-driver directory not found: $package"
}

$manifestPath = Join-Path $package 'verification.json'
if (-not (Test-Path $manifestPath -PathType Leaf)) {
  throw 'Staged virtual-driver installation requires verification.json created by stage-signed-virtual-driver.ps1.'
}
$verification = Get-Content $manifestPath -Raw | ConvertFrom-Json
if ([string]$verification.releaseChannel -notin @('pilot', 'retail')) {
  throw 'verification.json has an invalid virtual-driver release channel.'
}
if ([string]$verification.architecture -notin @('x64', 'ARM64')) {
  throw 'verification.json has an invalid virtual-driver architecture.'
}
Assert-StagedArchitecture -PackageArchitecture ([string]$verification.architecture)

$inf = Join-Path $package 'VoxveilVirtualAudio.inf'
$cat = Join-Path $package 'VoxveilVirtualAudio.cat'
$sys = Join-Path $package 'VoxveilVirtualAudio.sys'
Assert-StagedFileHash $inf ([string]$verification.infSha256) 'VoxveilVirtualAudio.inf'
Assert-StagedFileHash $cat ([string]$verification.catalogSha256) 'VoxveilVirtualAudio.cat'
Assert-StagedFileHash $sys ([string]$verification.driverSha256) 'VoxveilVirtualAudio.sys'

$infText = Get-Content $inf -Raw
if ($infText -notmatch '(?im)Root\\VoxveilVirtualAudio') {
  throw 'Staged virtual-driver INF does not contain Root\VoxveilVirtualAudio.'
}
if ($infText -notmatch '(?i)79E4E58C-9714-44E8-ACA1-426F24B7A1E9') {
  throw 'Staged virtual-driver INF does not contain the fixed Voxveil device-interface identity.'
}
if ($infText -notmatch '(?im)^\s*CatalogFile\s*=\s*VoxveilVirtualAudio\.cat\s*$') {
  throw 'Staged virtual-driver INF does not reference VoxveilVirtualAudio.cat.'
}
if ($infText -match '(?i)TESTSIGNING|test certificate|Sysvad_|Tablet Audio Sample|Contoso|SwapAPO|DelayAPO') {
  throw 'Staged virtual-driver INF contains a development or Microsoft-sample identity.'
}

$catalogSignature = Get-AuthenticodeSignature $cat
if ($catalogSignature.Status -ne 'Valid' -or -not $catalogSignature.SignerCertificate) {
  throw 'Staged virtual-driver catalog does not have a valid Authenticode signature.'
}
$signerText = $catalogSignature.SignerCertificate.Subject + ' ' + $catalogSignature.SignerCertificate.Issuer
if ($signerText -notmatch '(?i)Microsoft') {
  throw "Staged virtual-driver catalog signer is not identified as Microsoft: $($catalogSignature.SignerCertificate.Subject)"
}

$deviceHelper = Join-Path $PSScriptRoot 'voxveil-virtual-device.exe'
if (-not (Test-Path $deviceHelper -PathType Leaf)) {
  throw "Voxveil root-device helper is missing: $deviceHelper"
}

$statePath = Join-Path $package 'virtual-driver-install-state.json'
$currentBootMarker = Get-WindowsBootMarker
if (Test-Path $statePath -PathType Leaf) {
  $previousState = Get-Content $statePath -Raw | ConvertFrom-Json
  $pendingProperty = $previousState.PSObject.Properties['pendingReboot']
  $bootMarkerProperty = $previousState.PSObject.Properties['pendingRebootBootMarker']
  $uninstallCompleteProperty = $previousState.PSObject.Properties['uninstallComplete']
  $previousPendingReboot = $pendingProperty -and [bool]$pendingProperty.Value
  $previousBootMarker = if ($bootMarkerProperty) { [string]$bootMarkerProperty.Value } else { '' }
  $previousUninstallComplete = $uninstallCompleteProperty -and [bool]$uninstallCompleteProperty.Value
  if ($previousPendingReboot -and $previousBootMarker -and $previousBootMarker -eq $currentBootMarker) {
    throw 'Restart Windows before continuing the Voxveil virtual-driver installation.'
  }

  if ($previousUninstallComplete) {
    Assert-CompletedUninstallAbsent $previousState
  } else {
    $previousInfSha256 = [string]$previousState.infSha256
    $previousCatalogSha256 = [string]$previousState.catalogSha256
    $previousDriverSha256 = [string]$previousState.driverSha256
    foreach ($recordedHash in @($previousInfSha256, $previousCatalogSha256, $previousDriverSha256)) {
      if ($recordedHash -notmatch '^[0-9A-Fa-f]{64}$') {
        throw 'Existing virtual-driver install state does not contain a complete signed-package identity. Uninstall the recorded Voxveil Virtual Audio package before installing again.'
      }
    }
    if ($previousInfSha256.ToLowerInvariant() -ne ([string]$verification.infSha256).ToLowerInvariant() -or
        $previousCatalogSha256.ToLowerInvariant() -ne ([string]$verification.catalogSha256).ToLowerInvariant() -or
        $previousDriverSha256.ToLowerInvariant() -ne ([string]$verification.driverSha256).ToLowerInvariant()) {
      throw 'Uninstall the currently recorded Voxveil Virtual Audio package before installing a different signed package.'
    }
  }
}

$deviceInstanceId = $null
function Write-VirtualDriverInstallState {
  param(
    [Parameter(Mandatory = $true)][string]$PublishedInf,
    [bool]$PendingReboot = $false,
    [bool]$UninstallComplete = $false
  )

  if ($PublishedInf -notmatch '^oem\d+\.inf$') {
    throw "Cannot record virtual-driver install state for invalid published INF '$PublishedInf'."
  }
  if (-not $deviceInstanceId -or $deviceInstanceId -match '[\r\n]') {
    throw 'Cannot record virtual-driver install state without the exact devnode instance ID.'
  }

  $pendingRebootBootMarker = if ($PendingReboot) { $currentBootMarker } else { $null }
  @{
    publishedInf = $PublishedInf
    deviceInstanceId = $deviceInstanceId
    releaseChannel = [string]$verification.releaseChannel
    architecture = [string]$verification.architecture
    infSha256 = ([string]$verification.infSha256).ToLowerInvariant()
    catalogSha256 = ([string]$verification.catalogSha256).ToLowerInvariant()
    driverSha256 = ([string]$verification.driverSha256).ToLowerInvariant()
    pendingReboot = $PendingReboot
    pendingRebootBootMarker = $pendingRebootBootMarker
    uninstallComplete = $UninstallComplete
  } | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8
}

function Write-VirtualDriverRebootTombstone {
  @{
    pendingReboot = $true
    pendingRebootBootMarker = $currentBootMarker
    uninstallComplete = $true
  } | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8
}

$beforePublishedInfNames = @(Get-VoxveilPublishedInfNames)
$newPublishedInf = $null
$deviceCreated = $false
try {
  Write-Host "Preparing Root\VoxveilVirtualAudio devnode..."
  $ensureOutput = @(& $deviceHelper ensure $inf)
  if ($LASTEXITCODE -ne 0) {
    throw "voxveil-virtual-device.exe failed to ensure the root devnode (exit $LASTEXITCODE)."
  }
  $deviceInstanceId = Get-HelperValue $ensureOutput 'instanceId'
  $createdValue = Get-HelperValue $ensureOutput 'created'
  $helperRebootValue = Get-HelperValue $ensureOutput 'rebootRequired'
  if (-not $deviceInstanceId -or $deviceInstanceId -match '[\r\n]' -or
      $createdValue -notin @('0', '1') -or $helperRebootValue -notin @('0', '1')) {
    throw 'voxveil-virtual-device.exe returned invalid ensure metadata.'
  }
  $deviceCreated = $createdValue -eq '1'
  $helperRebootRequired = $helperRebootValue -eq '1'

  Write-Host "Installing verified Voxveil virtual driver ($($verification.releaseChannel), $($verification.architecture))..."
  pnputil.exe /add-driver $inf /install | Out-Host
  $pnputilExitCode = $LASTEXITCODE

  $afterPublishedInfNames = @(Get-VoxveilPublishedInfNames)
  $newPublishedInfNames = @($afterPublishedInfNames | Where-Object { $beforePublishedInfNames -inotcontains $_ })
  if ($newPublishedInfNames.Count -gt 1) {
    throw "PnPUtil added multiple new Voxveil driver-store packages unexpectedly: $($newPublishedInfNames -join ', ')."
  }
  if ($newPublishedInfNames.Count -eq 1) {
    $newPublishedInf = [string]$newPublishedInfNames[0]
  }

  $installedDrivers = @(Get-CimInstance Win32_PnPSignedDriver |
    Where-Object {
      $_.DeviceID -ieq $deviceInstanceId -and
      $_.DriverProviderName -eq 'Voxveil' -and
      $_.DeviceName -eq 'Voxveil Virtual Audio' -and
      $_.InfName -match '^oem\d+\.inf$'
    })

  if ($pnputilExitCode -ne 0) {
    if ($pnputilExitCode -ne 3010) {
      throw "PnPUtil failed to install VoxveilVirtualAudio.inf (exit $pnputilExitCode)."
    }
  }

  $lifecycleRebootRequired = $helperRebootRequired -or $pnputilExitCode -eq 3010
  if ($lifecycleRebootRequired) {
    if ($newPublishedInf) {
      $publishedInf = $newPublishedInf
    } elseif ($installedDrivers.Count -eq 1) {
      $publishedInf = [string]$installedDrivers[0].InfName
    } else {
      throw "Virtual-device or package installation requires a restart, but the exact Voxveil driver package could not be resolved safely (new packages=$($newPublishedInfNames.Count), bindings=$($installedDrivers.Count))."
    }
    Write-VirtualDriverInstallState -PublishedInf $publishedInf -PendingReboot $true
    Write-Warning 'Voxveil Virtual Audio was staged successfully, but Windows requires a restart before the driver binding can be verified. Restart Windows before using or reinstalling the virtual driver.'
    exit 3010
  }

  if ($installedDrivers.Count -ne 1) {
    throw "Installed Voxveil Virtual Audio devnode could not be resolved to exactly one signed driver binding (found $($installedDrivers.Count))."
  }
  $publishedInf = [string]$installedDrivers[0].InfName
  if ($newPublishedInf -and $publishedInf -ine $newPublishedInf) {
    throw "Newly added driver-store package '$newPublishedInf' does not match the package bound to the Voxveil devnode '$publishedInf'."
  }

  Write-VirtualDriverInstallState -PublishedInf $publishedInf

  Write-Host "Voxveil Virtual Audio installed as $publishedInf on $deviceInstanceId."
  Write-Host 'Use Windows Sound settings to select Voxveil Input when the relay path is desired.'
}
catch {
  $devnodeRollbackSucceeded = -not $deviceCreated
  $rollbackHelperRebootRequired = $false
  if ($deviceCreated -and $deviceInstanceId) {
    Write-Warning "Rolling back newly created Voxveil devnode $deviceInstanceId after installation failure."
    $rollbackRemoveOutput = @(& $deviceHelper remove $deviceInstanceId)
    $rollbackRemoveExitCode = $LASTEXITCODE
    $rollbackRemoveOutput | Out-Host
    if ($rollbackRemoveExitCode -ne 0) {
      Write-Warning "Devnode rollback failed with exit code $rollbackRemoveExitCode; manual cleanup may be required."
    } else {
      $rollbackRebootValue = Get-HelperValue $rollbackRemoveOutput 'rebootRequired'
      if ($rollbackRebootValue -notin @('0', '1')) {
        Write-Warning 'Devnode rollback returned invalid reboot metadata; package rollback was skipped to preserve ownership for manual cleanup.'
      } else {
        $rollbackHelperRebootRequired = $rollbackRebootValue -eq '1'
        $devnodeRollbackSucceeded = $true
      }
    }
  }

  if ($devnodeRollbackSucceeded -and $newPublishedInf) {
    Write-Warning "Rolling back newly added Voxveil driver-store package $newPublishedInf after installation failure."
    pnputil.exe /delete-driver $newPublishedInf | Out-Host
    $rollbackDeleteExitCode = $LASTEXITCODE
    if ($rollbackDeleteExitCode -ne 0 -and $rollbackDeleteExitCode -ne 3010) {
      Write-VirtualDriverInstallState -PublishedInf $newPublishedInf -PendingReboot $rollbackHelperRebootRequired
      Write-Warning "Driver-store rollback failed for $newPublishedInf with exit code $rollbackDeleteExitCode; ownership state was kept for scoped recovery."
    } else {
      $rollbackLifecycleRebootRequired = $rollbackHelperRebootRequired -or $rollbackDeleteExitCode -eq 3010
      if ($rollbackLifecycleRebootRequired) {
        Write-VirtualDriverInstallState -PublishedInf $newPublishedInf -PendingReboot $true -UninstallComplete $true
        Write-Warning "Rollback removed $newPublishedInf successfully, but Windows requires a restart before another Voxveil virtual-driver lifecycle mutation."
      }
    }
  } elseif ($newPublishedInf) {
    Write-VirtualDriverInstallState -PublishedInf $newPublishedInf
    Write-Warning "New driver-store package $newPublishedInf remains installed because the devnode could not be safely rolled back; ownership state was kept for scoped recovery."
  } elseif ($rollbackHelperRebootRequired) {
    Write-VirtualDriverRebootTombstone
    Write-Warning 'Devnode rollback requires a Windows restart before retrying the Voxveil virtual-driver installation.'
  }
  throw
}