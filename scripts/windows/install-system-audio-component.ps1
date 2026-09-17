[CmdletBinding(DefaultParameterSetName = 'Descriptor')]
param(
  [Parameter(ParameterSetName = 'Descriptor', Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$EndpointDescriptor,

  [Parameter(ParameterSetName = 'Manual', Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$HardwareId,

  [Parameter(ParameterSetName = 'Manual', Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$ReferenceString,

  [Parameter(ParameterSetName = 'Manual')]
  [switch]$TestSign
)

$ErrorActionPreference = 'Stop'

function Assert-Administrator {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($identity)
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this script from an elevated PowerShell (Run as administrator).'
  }
}

function Find-WdkTool([string]$Name) {
  $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
  if (-not (Test-Path $kits)) { return $null }
  Get-ChildItem $kits -Recurse -Filter $Name -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1 -ExpandProperty FullName
}

function Get-OptionalProperty($Object, [string]$Name) {
  $property = $Object.PSObject.Properties[$Name]
  if ($property) { return $property.Value }
  return $null
}

function Get-VoxveilPublishedInfNames {
  @(Get-CimInstance Win32_PnPSignedDriver |
    Where-Object { $_.DriverProviderName -eq 'Voxveil' -and $_.InfName -match '^oem\d+\.inf$' } |
    Select-Object -ExpandProperty InfName -Unique)
}

function Assert-StagedFileHash([string]$Path, [string]$ExpectedSha256, [string]$Description) {
  if (-not (Test-Path $Path -PathType Leaf)) {
    throw "Verified production APO artifact is missing: $Description ($Path)"
  }
  if ($ExpectedSha256 -notmatch '^[0-9A-Fa-f]{64}$') {
    throw "apo-verification.json contains an invalid SHA-256 value for $Description."
  }
  $actual = (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $ExpectedSha256.ToLowerInvariant()) {
    throw "Verified production APO artifact changed after staging: $Description."
  }
}

function Assert-StagedProductionApo([string]$Root) {
  $manifestPath = Join-Path $Root 'apo-verification.json'
  if (-not (Test-Path $manifestPath -PathType Leaf)) {
    throw 'Production install requires apo-verification.json created by the signed APO staging gate.'
  }
  $verification = Get-Content $manifestPath -Raw | ConvertFrom-Json
  if ([string]$verification.extensionId -ine '1D81E93D-AB81-473B-9E5E-94FAE8D2377F') {
    throw 'apo-verification.json does not match the committed Voxveil extension servicing lineage.'
  }
  if ([string]$verification.capxContext -ine '63E268CE-4CBC-48E0-BEB6-55103316F477') {
    throw 'apo-verification.json does not match the committed Voxveil CAPX property context.'
  }

  $artifacts = @(
    @('VoxveilApo.inf', [string]$verification.apoInfSha256),
    @('VoxveilApo.dll', [string]$verification.apoDllSha256),
    @('VoxveilApo.cat', [string]$verification.apoCatalogSha256),
    @('VoxveilApoExtension.inf', [string]$verification.extensionInfSha256),
    @('VoxveilApoExtension.cat', [string]$verification.extensionCatalogSha256)
  )
  foreach ($artifact in $artifacts) {
    Assert-StagedFileHash (Join-Path $Root $artifact[0]) $artifact[1] $artifact[0]
  }
  return $verification
}

function Resolve-EndpointDescriptor([string]$DescriptorPath, [string]$Root) {
  if (-not (Test-Path $DescriptorPath -PathType Leaf)) {
    throw "device-changed: endpoint descriptor no longer exists: $DescriptorPath"
  }
  $descriptor = Get-Content $DescriptorPath -Raw | ConvertFrom-Json
  foreach ($name in @('endpointId', 'bindingPnpInstanceId', 'pnpInstanceId', 'hardwareId', 'driverInf')) {
    if (-not $descriptor.$name) { throw "device-changed: endpoint descriptor is missing $name" }
  }

  $topologyInterfacePath = [string](Get-OptionalProperty $descriptor 'topologyInterfacePath')
  $audioInterfacePath = [string](Get-OptionalProperty $descriptor 'audioInterfacePath')
  $topologyReference = [string](Get-OptionalProperty $descriptor 'topologyReference')
  if ([bool]$topologyInterfacePath -ne [bool]$audioInterfacePath) {
    throw 'device-changed: endpoint descriptor has an incomplete runtime interface binding.'
  }
  $runtimeBound = [bool]$topologyInterfacePath -and [bool]$audioInterfacePath
  if (-not $runtimeBound -and -not $topologyReference) {
    throw 'device-changed: endpoint descriptor has neither runtime interfaces nor a legacy topology reference.'
  }

  $helper = Join-Path $Root 'discover-system-audio-endpoints.ps1'
  if (-not (Test-Path $helper -PathType Leaf)) {
    throw "Required endpoint discovery helper not found: $helper"
  }
  $request = ConvertTo-Json -InputObject @([pscustomobject]@{
    endpointId = [string]$descriptor.endpointId
    displayName = ''
    isDefault = $false
    runtimeDeviceId = [string]$descriptor.bindingPnpInstanceId
    runtimeAliasMatch = $runtimeBound
  }) -Compress
  $output = $request | & powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File $helper
  if ($LASTEXITCODE -ne 0) { throw 'device-changed: endpoint discovery failed during elevated revalidation.' }
  $parsedResolved = ConvertFrom-Json ($output -join [Environment]::NewLine)
  $resolvedItems = [Collections.Generic.List[object]]::new()
  foreach ($resolvedItem in $parsedResolved) {
    $resolvedItems.Add($resolvedItem)
  }
  if ($resolvedItems.Count -ne 1) { throw 'device-changed: selected endpoint could not be uniquely re-resolved.' }
  $resolved = $resolvedItems[0]

  if ([string]$resolved.bindingPnpInstanceId -ine [string]$descriptor.bindingPnpInstanceId) {
    throw 'device-changed: the playback endpoint now maps to a different topology binding device.'
  }
  if ([string]$resolved.pnpInstanceId -ine [string]$descriptor.pnpInstanceId) {
    throw 'device-changed: the playback endpoint driver metadata now maps to a different PnP device.'
  }
  if ([string]$resolved.driverInf -ine [string]$descriptor.driverInf) {
    throw 'device-changed: the playback endpoint driver changed after discovery.'
  }
  $currentHardware = @($resolved.hardwareIds | ForEach-Object { [string]$_ })
  if (-not ($currentHardware -icontains [string]$descriptor.hardwareId)) {
    throw 'device-changed: the playback endpoint hardware IDs changed after discovery.'
  }

  if (-not $runtimeBound) {
    $currentTopology = @($resolved.topologyReferences | ForEach-Object { [string]$_ })
    if (-not ($currentTopology | Where-Object { $_ -ieq $topologyReference })) {
      throw 'device-changed: the playback endpoint topology binding changed.'
    }
  }

  return [pscustomobject]@{
    EndpointId = [string]$descriptor.endpointId
    BindingPnpInstanceId = [string]$descriptor.bindingPnpInstanceId
    HardwareId = [string]$descriptor.hardwareId
    RuntimeBound = $runtimeBound
    TopologyInterfacePath = $topologyInterfacePath
    AudioInterfacePath = $audioInterfacePath
    ReferenceString = $topologyReference
  }
}

Assert-Administrator
if ($PSCmdlet.ParameterSetName -eq 'Manual' -and -not $TestSign) {
  throw 'Manual HardwareId/ReferenceString mode is development-only and requires -TestSign.'
}

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$statePath = Join-Path $root 'install-state.json'
$previousInstalledInfNames = @()
if (Test-Path $statePath -PathType Leaf) {
  $previousState = Get-Content $statePath -Raw | ConvertFrom-Json
  $previousInstalledInfNames = @($previousState.installedInfNames) |
    Where-Object { $_ -match '^oem\d+\.inf$' }
}
$beforeInstalledInfNames = @(Get-VoxveilPublishedInfNames)

$selectedEndpointId = $null
$bindingPnpInstanceId = $null
$topologyInterfacePath = $null
$audioInterfacePath = $null
$runtimeBound = $false
if ($PSCmdlet.ParameterSetName -eq 'Descriptor') {
  $binding = Resolve-EndpointDescriptor $EndpointDescriptor $root
  $selectedEndpointId = $binding.EndpointId
  $bindingPnpInstanceId = $binding.BindingPnpInstanceId
  $HardwareId = $binding.HardwareId
  $runtimeBound = $binding.RuntimeBound
  $topologyInterfacePath = $binding.TopologyInterfacePath
  $audioInterfacePath = $binding.AudioInterfacePath
  $ReferenceString = $binding.ReferenceString
}

$apoInf = Join-Path $root 'VoxveilApo.inf'
$apoDll = Join-Path $root 'VoxveilApo.dll'
$template = Join-Path $root 'VoxveilApoExtension.inf.template'
$generator = Join-Path $root 'new-apo-extension-inf.ps1'
$control = Join-Path $root 'voxveil-control.exe'
$useLegacyRuntimeAttachment = $TestSign -and $runtimeBound
$bindingMode = if (-not $TestSign) {
  'capx-extension'
} elseif ($useLegacyRuntimeAttachment) {
  'legacy-runtime-interface'
} else {
  'legacy-reference'
}

function Write-InstallStateSnapshot {
  $afterInstalledInfNames = @(Get-VoxveilPublishedInfNames)
  $newInstalledInfNames = @($afterInstalledInfNames | Where-Object { $beforeInstalledInfNames -inotcontains $_ })
  $previousStillInstalledInfNames = @($previousInstalledInfNames | Where-Object { $afterInstalledInfNames -icontains $_ })
  $installed = @($previousStillInstalledInfNames + $newInstalledInfNames)
  $installed = @($installed | Sort-Object -Unique)

  @{
    installedInfNames = @($installed)
    endpointId = $selectedEndpointId
    hardwareId = $HardwareId
    bindingMode = $bindingMode
    bindingPnpInstanceId = $bindingPnpInstanceId
    topologyInterfacePath = $topologyInterfacePath
    audioInterfacePath = $audioInterfacePath
    referenceString = $ReferenceString
  } | ConvertTo-Json -Depth 3 | Set-Content $statePath -Encoding utf8
}

foreach ($required in @($apoInf, $apoDll)) {
  if (-not (Test-Path $required)) { throw "Required system-audio file not found: $required" }
}

$work = Join-Path $env:TEMP ("voxveil-apo-install-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $work | Out-Null
try {
  Copy-Item $apoInf, $apoDll -Destination $work
  $extensionInf = Join-Path $work 'VoxveilApoExtension.inf'

  if ($TestSign) {
    foreach ($required in @($template, $generator)) {
      if (-not (Test-Path $required)) { throw "Required development packaging file not found: $required" }
    }
    if ($runtimeBound) {
      & $generator -HardwareId $HardwareId -FxPropertyMode Legacy -TemplatePath $template -OutputPath $extensionInf
    } else {
      & $generator -HardwareId $HardwareId -ReferenceString $ReferenceString -FxPropertyMode Legacy -TemplatePath $template -OutputPath $extensionInf
    }

    $inf2cat = Find-WdkTool 'Inf2Cat.exe'
    $signtool = Find-WdkTool 'signtool.exe'
    if (-not $inf2cat -or -not $signtool) {
      throw 'TestSign requires the Windows Driver Kit (Inf2Cat.exe and signtool.exe). Install the WDK first.'
    }

    $testSigning = (bcdedit /enum '{current}' 2>$null | Select-String -Pattern '^testsigning\s+Yes$')
    if (-not $testSigning) {
      Write-Warning 'Windows TESTSIGNING is not enabled. The development package may install but AudioDG/PNP will not load it after reboot.'
      Write-Warning 'Enable TESTSIGNING only on a dedicated development machine; Secure Boot may need to be disabled.'
    }

    $certificate = New-SelfSignedCertificate `
      -Type CodeSigningCert `
      -Subject 'CN=Voxveil Development APO' `
      -CertStoreLocation 'Cert:\LocalMachine\My' `
      -KeyExportPolicy Exportable `
      -HashAlgorithm SHA256 `
      -NotAfter (Get-Date).AddYears(2)

    $cerPath = Join-Path $work 'VoxveilDevelopment.cer'
    Export-Certificate -Cert $certificate -FilePath $cerPath | Out-Null
    certutil.exe -addstore -f Root $cerPath | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Failed to trust the Voxveil development certificate in LocalMachine\Root.' }
    certutil.exe -addstore -f TrustedPublisher $cerPath | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Failed to trust the Voxveil development certificate in LocalMachine\TrustedPublisher.' }

    & $signtool sign /fd SHA256 /sha1 $certificate.Thumbprint /s My /sm (Join-Path $work 'VoxveilApo.dll') | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Failed to test-sign VoxveilApo.dll.' }

    & $inf2cat "/driver:$work" /os:10_X64 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Inf2Cat rejected the Voxveil APO package.' }

    Get-ChildItem $work -Filter '*.cat' -File | ForEach-Object {
      & $signtool sign /fd SHA256 /sha1 $certificate.Thumbprint /s My /sm $_.FullName | Out-Host
      if ($LASTEXITCODE -ne 0) { throw "Failed to sign catalog: $($_.Name)" }
    }
  } else {
    $null = Assert-StagedProductionApo $root
    $prebuiltExtension = Join-Path $root 'VoxveilApoExtension.inf'
    $apoCat = Join-Path $root 'VoxveilApo.cat'
    $extensionCat = Join-Path $root 'VoxveilApoExtension.cat'

    $prebuiltText = Get-Content $prebuiltExtension -Raw
    if (-not $prebuiltText.Contains($HardwareId)) {
      throw 'The signed Voxveil Extension INF does not match the automatically resolved playback endpoint hardware ID.'
    }
    if ($prebuiltText -notmatch '(?im)^\s*ExtensionId\s*=\s*\{1D81E93D-AB81-473B-9E5E-94FAE8D2377F\}\s*$') {
      throw 'The signed Voxveil Extension INF does not match the committed Voxveil extension servicing lineage.'
    }
    if ($prebuiltText -notmatch '(?i)VOXVEIL_APO_CONTEXT\s*=\s*"\{63E268CE-4CBC-48E0-BEB6-55103316F477\}"') {
      throw 'The signed Voxveil Extension INF is not the expected CAPX production package: the fixed property-context identity is missing.'
    }
    if ($prebuiltText -notmatch '(?im)^\s*HKR\s*,\s*FX\\0\\%VOXVEIL_APO_CONTEXT%\s*,\s*%PKEY_FX_Association%') {
      throw 'The signed Voxveil Extension INF is not CAPX production-bound to the Voxveil property context.'
    }
    if ($prebuiltText -match '(?im)^\s*HKR\s*,\s*FX\\0\s*,\s*%PKEY_FX_Association%') {
      throw 'The signed Voxveil Extension INF contains the legacy root FX association; production CAPX packages must not mix legacy and context property stores.'
    }
    if ($prebuiltText -notmatch '(?im)^\s*AddInterface\s*=') {
      throw 'The signed Voxveil Extension INF has no signed endpoint interface binding. Runtime registry attachment is not a CAPX production path.'
    }
    if ($ReferenceString -and -not $prebuiltText.Contains($ReferenceString)) {
      throw 'The signed Voxveil Extension INF does not match the resolved playback endpoint topology reference.'
    }

    Copy-Item $prebuiltExtension $extensionInf
    Copy-Item $apoCat, $extensionCat -Destination $work
  }

  Write-Host 'Staging/installing the Voxveil APO software-component package...'
  pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install | Out-Host
  if ($LASTEXITCODE -ne 0) { throw "PnPUtil failed to stage VoxveilApo.inf (exit $LASTEXITCODE)." }
  Write-InstallStateSnapshot

  Write-Host 'Installing the endpoint-specific Voxveil Extension INF...'
  pnputil.exe /add-driver $extensionInf /install | Out-Host
  if ($LASTEXITCODE -ne 0) { throw "PnPUtil failed to install VoxveilApoExtension.inf (exit $LASTEXITCODE)." }
  Write-InstallStateSnapshot

  if ($useLegacyRuntimeAttachment) {
    if (-not (Test-Path $control -PathType Leaf)) {
      throw 'Legacy development runtime interface binding requires voxveil-control.exe in the packaged system-audio directory.'
    }
    Write-Warning 'Applying legacy development FX\\0 registry attachment. This is not the Windows 11 CAPX production path.'
    & $control attach-effects $bindingPnpInstanceId $topologyInterfacePath $audioInterfacePath | Out-Host
    if ($LASTEXITCODE -ne 0) {
      throw "Legacy runtime interface FX attachment failed (exit $LASTEXITCODE)."
    }
  }

  Write-Host 'Restarting Windows Audio so AudioDG rebuilds the endpoint graph...'
  Restart-Service Audiosrv -Force
  Start-Sleep -Seconds 2
  Write-InstallStateSnapshot

  if (Test-Path $control) {
    $status = & $control status 2>&1
    Write-Host "APO control status: $status"
    if ($LASTEXITCODE -ne 0 -or $status -notmatch 'loaded=[1-9][0-9]*') {
      throw 'installed-not-loaded: the package installed, but AudioDG did not load a real Voxveil processing instance on the selected playback endpoint.'
    }
  } elseif (-not $TestSign) {
    throw 'installed-not-loaded: production CAPX installation requires voxveil-control.exe so AudioDG load verification cannot be skipped.'
  } else {
    Write-Warning 'voxveil-control.exe was not present, so AudioDG load verification was skipped for this development/test installation.'
  }

  if ($TestSign) {
    Write-Host 'Voxveil development/test APO installed. This is not a production qualification result.'
  } else {
    Write-Host 'Voxveil production-signed CAPX APO package installed and bound to the selected render endpoint.'
  }
}
finally {
  Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
