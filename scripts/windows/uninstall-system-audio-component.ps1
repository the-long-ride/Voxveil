[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw 'Run this script from an elevated PowerShell (Run as administrator).'
}

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$devcon = Join-Path $root 'devcon.exe'
if (-not (Test-Path $devcon)) {
  throw "Bundled DevCon was not found: $devcon"
}

& $devcon remove 'Root\Sysvad_ComponentizedAudioSample'
if ($LASTEXITCODE -ne 0) {
  throw "DevCon failed with exit code $LASTEXITCODE."
}

Write-Host 'Voxveil development system-audio device removed.'
