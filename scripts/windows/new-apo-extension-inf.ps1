[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$HardwareId,

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
if ($ReferenceString -and $ReferenceString -match '[\r\n"]') {
  throw 'ReferenceString contains characters that are unsafe in an INF string value.'
}

$legacyAssociation = ''
$referenceDefinition = ''
if ($ReferenceString) {
  $legacyAssociation = @'
[DeviceExtensions.I.APO]
AddReg = APO.I.Association0.AddReg

[APO.I.Association0.AddReg]
HKR,FX\0,%PKEY_FX_Association%,,%KSNODETYPE_ANY%
HKR,FX\0,%PKEY_CompositeFX_StreamEffectClsid%,0x00010000,%VOXVEIL_SFX_CLSID%
HKR,FX\0,%PKEY_SFX_ProcessingModes_Supported_For_Streaming%,%REG_MULTI_SZ%,%AUDIO_SIGNALPROCESSINGMODE_DEFAULT%,%AUDIO_SIGNALPROCESSINGMODE_MEDIA%,%AUDIO_SIGNALPROCESSINGMODE_MOVIE%

[DeviceExtension_Install.Interfaces]
AddInterface = %KSCATEGORY_AUDIO%, %REFERENCE_STRING%, DeviceExtensions.I.APO
AddInterface = %KSCATEGORY_TOPOLOGY%, %REFERENCE_STRING%, DeviceExtensions.I.APO
'@
  $referenceDefinition = 'REFERENCE_STRING     = "' + $ReferenceString + '"'
}

$extensionId = [Guid]::NewGuid().ToString().ToUpperInvariant()
$content = Get-Content $TemplatePath -Raw
$content = $content.Replace('@@HARDWARE_ID@@', $HardwareId)
$content = $content.Replace('@@EXTENSION_ID@@', $extensionId)
$content = $content.Replace('@@LEGACY_INTERFACE_ASSOCIATION@@', $legacyAssociation.TrimEnd())
$content = $content.Replace('@@REFERENCE_STRING_DEFINITION@@', $referenceDefinition)

$directory = Split-Path -Parent $OutputPath
if ($directory) {
  New-Item -ItemType Directory -Force -Path $directory | Out-Null
}
Set-Content -Path $OutputPath -Value $content -Encoding ascii

Write-Host "Generated endpoint extension INF: $OutputPath"
Write-Host "  Hardware ID:  $HardwareId"
if ($ReferenceString) {
  Write-Host "  Binding:      legacy INF interface reference $ReferenceString"
} else {
  Write-Host '  Binding:      runtime SetupAPI device interfaces'
}
Write-Host "  Extension ID: {$extensionId}"
