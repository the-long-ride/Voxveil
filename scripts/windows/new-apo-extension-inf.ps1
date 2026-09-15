[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$HardwareId,

  [string]$ReferenceString,

  [ValidateSet('Capx', 'Legacy')]
  [string]$FxPropertyMode = 'Capx',

  [string]$TemplatePath,
  [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$ExtensionId = '1D81E93D-AB81-473B-9E5E-94FAE8D2377F'
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
if ($FxPropertyMode -eq 'Capx' -and -not $ReferenceString) {
  throw 'CAPX generation requires a stable endpoint ReferenceString so the signed extension INF owns the FX property association.'
}

$association = ''
$referenceDefinition = ''
if ($ReferenceString) {
  $referenceDefinition = 'REFERENCE_STRING     = "' + $ReferenceString + '"'

  if ($FxPropertyMode -eq 'Capx') {
    $association = @'
[DeviceExtensions.I.APO]
AddReg = APO.I.Capx.AddReg

[APO.I.Capx.AddReg]
HKR,FX\0\%VOXVEIL_APO_CONTEXT%,%PKEY_FX_Association%,,%KSNODETYPE_ANY%
HKR,FX\0\%VOXVEIL_APO_CONTEXT%\User,,,
HKR,FX\0,%PKEY_CompositeFX_StreamEffectClsid%,0x00010000,%VOXVEIL_SFX_CLSID%
HKR,FX\0,%PKEY_SFX_ProcessingModes_Supported_For_Streaming%,%REG_MULTI_SZ%,%AUDIO_SIGNALPROCESSINGMODE_DEFAULT%,%AUDIO_SIGNALPROCESSINGMODE_MEDIA%,%AUDIO_SIGNALPROCESSINGMODE_MOVIE%

[DeviceExtension_Install.Interfaces]
AddInterface = %KSCATEGORY_AUDIO%, %REFERENCE_STRING%, DeviceExtensions.I.APO
AddInterface = %KSCATEGORY_TOPOLOGY%, %REFERENCE_STRING%, DeviceExtensions.I.APO
'@
  } else {
    $association = @'
[DeviceExtensions.I.APO]
AddReg = APO.I.Legacy.AddReg

[APO.I.Legacy.AddReg]
HKR,FX\0,%PKEY_FX_Association%,,%KSNODETYPE_ANY%
HKR,FX\0,%PKEY_CompositeFX_StreamEffectClsid%,0x00010000,%VOXVEIL_SFX_CLSID%
HKR,FX\0,%PKEY_SFX_ProcessingModes_Supported_For_Streaming%,%REG_MULTI_SZ%,%AUDIO_SIGNALPROCESSINGMODE_DEFAULT%,%AUDIO_SIGNALPROCESSINGMODE_MEDIA%,%AUDIO_SIGNALPROCESSINGMODE_MOVIE%

[DeviceExtension_Install.Interfaces]
AddInterface = %KSCATEGORY_AUDIO%, %REFERENCE_STRING%, DeviceExtensions.I.APO
AddInterface = %KSCATEGORY_TOPOLOGY%, %REFERENCE_STRING%, DeviceExtensions.I.APO
'@
  }
}

$content = Get-Content $TemplatePath -Raw
if ($content -notmatch [regex]::Escape("ExtensionId = {$ExtensionId}")) {
  throw "Extension INF template must keep the committed Voxveil ExtensionId {$ExtensionId}."
}
$content = $content.Replace('@@HARDWARE_ID@@', $HardwareId)
$content = $content.Replace('@@FX_PROPERTY_ASSOCIATION@@', $association.TrimEnd())
$content = $content.Replace('@@REFERENCE_STRING_DEFINITION@@', $referenceDefinition)

if ($content -match '@@[A-Z_]+@@') {
  throw 'Generated extension INF still contains an unresolved template marker.'
}

$directory = Split-Path -Parent $OutputPath
if ($directory) {
  New-Item -ItemType Directory -Force -Path $directory | Out-Null
}
Set-Content -Path $OutputPath -Value $content -Encoding ascii

Write-Host "Generated endpoint extension INF: $OutputPath"
Write-Host "  Hardware ID:  $HardwareId"
Write-Host "  FX mode:      $FxPropertyMode"
if ($ReferenceString) {
  Write-Host "  Binding:      signed INF interface reference $ReferenceString"
} else {
  Write-Host '  Binding:      no INF interface binding; legacy runtime SetupAPI mutation is required explicitly.'
}
Write-Host "  Extension ID: {$ExtensionId}"
