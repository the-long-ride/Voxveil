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

$beforePublishedInfNames = @(Get-VoxveilPublishedInfNames)
$newPublishedInf = $null
$deviceInstanceId = $null
$deviceCreated = $false
try {
  Write-Host "Preparing Root\VoxveilVirtualAudio devnode..."
  $ensureOutput = @(& $deviceHelper ensure $inf)
  if ($LASTEXITCODE -ne 0) {
    throw "voxveil-virtual-device.exe failed to ensure the root devnode (exit $LASTEXITCODE)."
  }
  $deviceInstanceId = Get-HelperValue $ensureOutput 'instanceId'
  $createdValue = Get-HelperValue $ensureOutput 'created'
  if (-not $deviceInstanceId -or $deviceInstanceId -match '[\r\n]' -or $createdValue -notin @('0', '1')) {
    throw 'voxveil-virtual-device.exe returned invalid ensure metadata.'
  }
  $deviceCreated = $createdValue -eq '1'

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
  if ($pnputilExitCode -ne 0) {
    throw "PnPUtil failed to install VoxveilVirtualAudio.inf (exit $pnputilExitCode)."
  }

  $installedDrivers = @(Get-CimInstance Win32_PnPSignedDriver |
    Where-Object {
      $_.DeviceID -ieq $deviceInstanceId -and
      $_.DriverProviderName -eq 'Voxveil' -and
      $_.DeviceName -eq 'Voxveil Virtual Audio' -and
      $_.InfName -match '^oem\d+\.inf$'
    })
  if ($installedDrivers.Count -ne 1) {
    throw "Installed Voxveil Virtual Audio devnode could not be resolved to exactly one signed driver binding (found $($installedDrivers.Count))."
  }
  $publishedInf = [string]$installedDrivers[0].InfName
  if ($newPublishedInf -and $publishedInf -ine $newPublishedInf) {
    throw "Newly added driver-store package '$newPublishedInf' does not match the package bound to the Voxveil devnode '$publishedInf'."
  }

  @{
    publishedInf = $publishedInf
    deviceInstanceId = $deviceInstanceId
    releaseChannel = [string]$verification.releaseChannel
    architecture = [string]$verification.architecture
    infSha256 = ([string]$verification.infSha256).ToLowerInvariant()
    catalogSha256 = ([string]$verification.catalogSha256).ToLowerInvariant()
    driverSha256 = ([string]$verification.driverSha256).ToLowerInvariant()
  } | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $package 'virtual-driver-install-state.json') -Encoding utf8

  Write-Host "Voxveil Virtual Audio installed as $publishedInf on $deviceInstanceId."
  Write-Host 'Use Windows Sound settings to select Voxveil Input when the relay path is desired.'
}
catch {
  $devnodeRollbackSucceeded = -not $deviceCreated
  if ($deviceCreated -and $deviceInstanceId) {
    Write-Warning "Rolling back newly created Voxveil devnode $deviceInstanceId after installation failure."
    & $deviceHelper remove $deviceInstanceId | Out-Host
    if ($LASTEXITCODE -ne 0) {
      Write-Warning "Devnode rollback failed with exit code $LASTEXITCODE; manual cleanup may be required."
    } else {
      $devnodeRollbackSucceeded = $true
    }
  }

  if ($devnodeRollbackSucceeded -and $newPublishedInf) {
    Write-Warning "Rolling back newly added Voxveil driver-store package $newPublishedInf after installation failure."
    pnputil.exe /delete-driver $newPublishedInf | Out-Host
    if ($LASTEXITCODE -ne 0) {
      Write-Warning "Driver-store rollback failed for $newPublishedInf with exit code $LASTEXITCODE; manual cleanup may be required."
    }
  } elseif ($newPublishedInf) {
    Write-Warning "New driver-store package $newPublishedInf remains installed because the devnode could not be safely rolled back; manual cleanup may be required."
  }
  throw
}
