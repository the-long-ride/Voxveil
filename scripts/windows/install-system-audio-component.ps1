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

function Resolve-EndpointDescriptor([string]$DescriptorPath, [string]$Root) {
  if (-not (Test-Path $DescriptorPath -PathType Leaf)) {
    throw "device-changed: endpoint descriptor no longer exists: $DescriptorPath"
  }
  $descriptor = Get-Content $DescriptorPath -Raw | ConvertFrom-Json
  foreach ($name in @('endpointId', 'bindingPnpInstanceId', 'pnpInstanceId', 'hardwareId', 'driverInf', 'topologyReference')) {
    if (-not $descriptor.$name) { throw "device-changed: endpoint descriptor is missing $name" }
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
    runtimeAliasMatch = $true
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
  $currentTopology = @($resolved.topologyReferences | ForEach-Object { [string]$_ })
  $expectedTopology = [string]$descriptor.topologyReference
  if (-not ($currentTopology | Where-Object { $_ -ieq $expectedTopology })) {
    throw 'device-changed: the playback endpoint topology binding changed.'
  }

  return [pscustomobject]@{
    EndpointId = [string]$descriptor.endpointId
    HardwareId = [string]$descriptor.hardwareId
    ReferenceString = [string]$descriptor.topologyReference
  }
}

Assert-Administrator

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$selectedEndpointId = $null
if ($PSCmdlet.ParameterSetName -eq 'Descriptor') {
  $binding = Resolve-EndpointDescriptor $EndpointDescriptor $root
  $selectedEndpointId = $binding.EndpointId
  $HardwareId = $binding.HardwareId
  $ReferenceString = $binding.ReferenceString
}

$apoInf = Join-Path $root 'VoxveilApo.inf'
$apoDll = Join-Path $root 'VoxveilApo.dll'
$template = Join-Path $root 'VoxveilApoExtension.inf.template'
$generator = Join-Path $root 'new-apo-extension-inf.ps1'
$control = Join-Path $root 'voxveil-control.exe'

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
    & $generator -HardwareId $HardwareId -ReferenceString $ReferenceString -TemplatePath $template -OutputPath $extensionInf

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
    $prebuiltExtension = Join-Path $root 'VoxveilApoExtension.inf'
    $apoCat = Join-Path $root 'VoxveilApo.cat'
    $extensionCat = Join-Path $root 'VoxveilApoExtension.cat'
    foreach ($required in @($prebuiltExtension, $apoCat, $extensionCat)) {
      if (-not (Test-Path $required)) {
        throw 'Production install requires a matching production-signed extension package for this audio driver.'
      }
    }

    $prebuiltText = Get-Content $prebuiltExtension -Raw
    if (-not $prebuiltText.Contains($HardwareId) -or -not $prebuiltText.Contains($ReferenceString)) {
      throw 'The signed Voxveil Extension INF does not match the automatically resolved playback endpoint.'
    }
    Copy-Item $prebuiltExtension $extensionInf
    Copy-Item $apoCat, $extensionCat -Destination $work
  }

  Write-Host 'Staging/installing the Voxveil APO software-component package...'
  pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install | Out-Host
  if ($LASTEXITCODE -ne 0) { throw "PnPUtil failed to stage VoxveilApo.inf (exit $LASTEXITCODE)." }

  Write-Host 'Installing the endpoint-specific Voxveil Extension INF...'
  pnputil.exe /add-driver $extensionInf /install | Out-Host
  if ($LASTEXITCODE -ne 0) { throw "PnPUtil failed to install VoxveilApoExtension.inf (exit $LASTEXITCODE)." }

  Write-Host 'Restarting Windows Audio so AudioDG rebuilds the endpoint graph...'
  Restart-Service Audiosrv -Force
  Start-Sleep -Seconds 2

  $installed = Get-CimInstance Win32_PnPSignedDriver |
    Where-Object { $_.DriverProviderName -eq 'Voxveil' -and $_.InfName } |
    Select-Object -ExpandProperty InfName -Unique
  @{
    installedInfNames = @($installed)
    endpointId = $selectedEndpointId
    hardwareId = $HardwareId
    referenceString = $ReferenceString
  } | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $root 'install-state.json') -Encoding utf8

  if (Test-Path $control) {
    $status = & $control status 2>&1
    Write-Host "APO control status: $status"
    if ($LASTEXITCODE -ne 0 -or $status -notmatch 'loaded=[1-9][0-9]*') {
      throw 'installed-not-loaded: the package installed, but AudioDG did not load VoxveilApo.dll on the selected playback endpoint.'
    }
  } else {
    Write-Warning 'voxveil-control.exe was not present, so AudioDG load verification was skipped.'
  }

  Write-Host 'Voxveil componentized APO installed and attached to the selected render endpoint.'
}
finally {
  Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
