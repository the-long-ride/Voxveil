[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

function Assert-Administrator {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($identity)
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this script from an elevated PowerShell (Run as administrator).'
  }
}

Assert-Administrator

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$driverDir = Join-Path $root 'driver'
$devcon = Join-Path $root 'devcon.exe'
$inf = Join-Path $driverDir 'ComponentizedAudioSample.inf'
$hardwareId = 'Root\Sysvad_ComponentizedAudioSample'

if (-not (Test-Path $devcon)) {
  throw "Bundled DevCon was not found: $devcon"
}
if (-not (Test-Path $inf)) {
  throw "Bundled SysVAD package was not found: $inf"
}

$testSigning = (bcdedit /enum '{current}' 2>$null | Select-String -Pattern '^testsigning\s+Yes$')
if (-not $testSigning) {
  Write-Warning 'This GitHub Actions audio component is test-signed and Windows will not load it in normal production mode.'
  Write-Warning 'For a development machine, enable TESTSIGNING and reboot first. Secure Boot may need to be disabled; BitLocker may need to be suspended.'
  Write-Warning 'For normal end-user builds, replace this package with a Microsoft production-signed Voxveil driver.'
}

$certificate = Get-ChildItem -Path $driverDir -Filter '*.cer' -File -ErrorAction SilentlyContinue | Select-Object -First 1
if ($certificate) {
  Write-Host "Trusting test certificate: $($certificate.Name)"
  certutil.exe -addstore -f Root $certificate.FullName | Out-Host
  if ($LASTEXITCODE -ne 0) { throw 'Failed to add the test certificate to LocalMachine\Root.' }
  certutil.exe -addstore -f TrustedPublisher $certificate.FullName | Out-Host
  if ($LASTEXITCODE -ne 0) { throw 'Failed to add the test certificate to LocalMachine\TrustedPublisher.' }
}

Write-Host 'Installing Voxveil development virtual render endpoint...'
& $devcon install $inf $hardwareId
if ($LASTEXITCODE -ne 0) {
  throw "DevCon failed with exit code $LASTEXITCODE. Check C:\Windows\INF\setupapi.dev.log for details."
}

Write-Host ''
Write-Host 'System-audio component installed.'
Write-Host 'Next: open Windows Sound settings and set the SYSVAD / Virtual Audio Device (WDM) - Tablet endpoint as the default output.'
Write-Host 'Then restart Voxveil. Voxveil captures that render endpoint with WASAPI loopback and sends the processed stream to your physical output.'
