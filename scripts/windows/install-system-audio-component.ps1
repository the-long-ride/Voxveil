[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$HardwareId,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$ReferenceString,

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

Assert-Administrator

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
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
      Write-Warning 'Enable TESTSIGNING on a dedicated development machine; Secure Boot may need to be disabled.'
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
        throw 'Production install requires a prebuilt device-specific VoxveilApoExtension.inf and its matching production-signed catalogs.'
      }
    }

    $prebuiltText = Get-Content $prebuiltExtension -Raw
    if (-not $prebuiltText.Contains($HardwareId) -or -not $prebuiltText.Contains($ReferenceString)) {
      throw 'The prebuilt production Extension INF does not match the requested HardwareId/ReferenceString.'
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
  @{ installedInfNames = @($installed); hardwareId = $HardwareId; referenceString = $ReferenceString } |
    ConvertTo-Json -Depth 3 |
    Set-Content (Join-Path $root 'install-state.json') -Encoding utf8

  if (Test-Path $control) {
    $status = & $control status 2>&1
    Write-Host "APO control status: $status"
    if ($LASTEXITCODE -ne 0 -or $status -notmatch 'loaded=[1-9][0-9]*') {
      throw 'The package installed, but AudioDG did not load VoxveilApo.dll on the target endpoint. Check C:\Windows\INF\setupapi.dev.log and the selected hardware/reference-string pair.'
    }
  } else {
    Write-Warning 'voxveil-control.exe was not present, so AudioDG load verification was skipped.'
  }

  Write-Host 'Voxveil componentized APO installed and attached to the selected render endpoint.'
}
finally {
  Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
