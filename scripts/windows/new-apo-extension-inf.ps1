[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$HardwareId,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$ReferenceString,

  [string]$TemplatePath,
  [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
if (-not $TemplatePath) {
  $TemplatePath = Join-Path $root 'native\windows\package\VoxveilApoExtension.inf.template'
  if (-not (Test-Path $TemplatePath)) {
    # Packaged layout: script is inside system-audio beside the template.
    $TemplatePath = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) 'VoxveilApoExtension.inf.template'
  }
}
if (-not $OutputPath) {
  $OutputPath = Join-Path (Split-Path -Parent $TemplatePath) 'VoxveilApoExtension.inf'
}
if (-not (Test-Path $TemplatePath)) {
  throw "Extension INF template not found: $TemplatePath"
}

if ($HardwareId -match '[\r\n,]') {
  throw 'HardwareId contains characters that are unsafe in an INF model line.'
}
if ($ReferenceString -match '[\r\n"]') {
  throw 'ReferenceString contains characters that are unsafe in an INF string value.'
}

$extensionId = [Guid]::NewGuid().ToString().ToUpperInvariant()
$content = Get-Content $TemplatePath -Raw
$content = $content.Replace('@@HARDWARE_ID@@', $HardwareId)
$content = $content.Replace('@@REFERENCE_STRING@@', $ReferenceString)
$content = $content.Replace('@@EXTENSION_ID@@', $extensionId)

$directory = Split-Path -Parent $OutputPath
if ($directory) {
  New-Item -ItemType Directory -Force -Path $directory | Out-Null
}
Set-Content -Path $OutputPath -Value $content -Encoding ascii

Write-Host "Generated endpoint extension INF: $OutputPath"
Write-Host "  Hardware ID:      $HardwareId"
Write-Host "  Reference string: $ReferenceString"
Write-Host "  Extension ID:     {$extensionId}"
