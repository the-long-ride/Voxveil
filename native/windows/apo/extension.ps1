Set-StrictMode -Version 2.0

function Assert-VoxveilInfString {
    param(
        [string]$Value,
        [string]$Label
    )
    if ($Value -match '[\r\n"]') {
        throw "$Label contains characters that cannot be represented safely in a Voxveil INF string."
    }
    return $Value
}

function New-VoxveilExtensionInf {
    param(
        [Parameter(Mandatory = $true)]
        [object[]]$Targets,
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if ($Targets.Count -ne 1) {
        throw "Voxveil requires exactly one current default render target; discovered $($Targets.Count)."
    }

    $target = $Targets[0]
    $refs = @($target.topologyRefs)
    if ($refs.Count -eq 0) {
        throw ("Target {0} has no topology interfaces." -f $target.instanceId)
    }

    $hardwareId = Assert-VoxveilInfString ([string]$target.hardwareId) 'default render hardware ID'
    $instanceId = Assert-VoxveilInfString ([string]$target.instanceId) 'default render instance ID'

    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add('[Version]')
    $lines.Add('Signature="$WINDOWS NT$"')
    $lines.Add('Class=Extension')
    $lines.Add('ClassGuid={e2f84ce7-8efa-411c-aa69-97454ca4cb57}')
    $lines.Add('ExtensionId={0D6F6D57-5BD4-4F45-9C40-6F70E34F0AA2}')
    $lines.Add('Provider=%ProviderName%')
    $lines.Add('DriverVer=08/19/2026,0.1.0.0')
    $lines.Add('CatalogFile=VoxveilExtension.cat')
    $lines.Add('PnpLockDown=1')
    $lines.Add('')
    $lines.Add('[Manufacturer]')
    $lines.Add('%MfgName%=DeviceExtensions,NTamd64.10.0...22000')
    $lines.Add('')
    $lines.Add('[DeviceExtensions.NTamd64.10.0...22000]')
    $lines.Add('%Device.Desc%=Device_Install,%Device.HardwareId%')
    $lines.Add('')
    $lines.Add('[Device_Install]')
    $lines.Add('')
    $lines.Add('[Device_Install.Components]')
    $lines.Add('AddComponent=VoxveilApo,,VoxveilApo_AddComponent')
    $lines.Add('')
    $lines.Add('[VoxveilApo_AddComponent]')
    $lines.Add('ComponentIDs=VEN_VOXV&CID_APO')
    $lines.Add('Description="Voxveil Audio Processing Object"')
    $lines.Add('')
    $lines.Add('[Device_Install.Interfaces]')
    for ($refIndex = 0; $refIndex -lt $refs.Count; $refIndex++) {
        $lines.Add(('AddInterface=%KSCATEGORY_AUDIO%,%Device.TopologyRef{0}%,VoxveilFx' -f $refIndex))
        $lines.Add(('AddInterface=%KSCATEGORY_TOPOLOGY%,%Device.TopologyRef{0}%,VoxveilFx' -f $refIndex))
    }
    $lines.Add('')
    $lines.Add('[VoxveilFx]')
    $lines.Add('AddReg=VoxveilFx.AddReg')
    $lines.Add('')
    $lines.Add('[VoxveilFx.AddReg]')
    $lines.Add('HKR,FX\0,%PKEY_FX_Association%,,%KSNODETYPE_ANY%')
    $lines.Add('HKR,FX\0,%PKEY_CompositeFX_EndpointEffectClsid%,0x00010008,%VOXVEIL_APO_CLSID%')
    $lines.Add('HKR,FX\0,%PKEY_EFX_ProcessingModes_Supported_For_Streaming%,0x00010008,%AUDIO_SIGNALPROCESSINGMODE_DEFAULT%')
    $lines.Add('')
    $lines.Add('[Strings]')
    $lines.Add('MfgName="Voxveil"')
    $lines.Add('ProviderName="Voxveil"')
    $lines.Add(('Device.Desc="Voxveil audio extension for {0}"' -f $instanceId))
    $lines.Add(('Device.HardwareId="{0}"' -f $hardwareId))
    $lines.Add('KSCATEGORY_AUDIO="{6994AD04-93EF-11D0-A3CC-00A0C9223196}"')
    $lines.Add('KSCATEGORY_TOPOLOGY="{DDA54A40-1E4C-11D1-A050-405705C10000}"')
    $lines.Add('PKEY_FX_Association="{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},0"')
    $lines.Add('PKEY_CompositeFX_EndpointEffectClsid="{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},15"')
    $lines.Add('PKEY_EFX_ProcessingModes_Supported_For_Streaming="{D3993A3F-99C2-4402-B5EC-A92A0367664B},7"')
    $lines.Add('AUDIO_SIGNALPROCESSINGMODE_DEFAULT="{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}"')
    $lines.Add('KSNODETYPE_ANY="{00000000-0000-0000-0000-000000000000}"')
    $lines.Add('VOXVEIL_APO_CLSID="{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}"')

    for ($refIndex = 0; $refIndex -lt $refs.Count; $refIndex++) {
        $reference = Assert-VoxveilInfString ([string]$refs[$refIndex]) ("default render topology reference {0}" -f $refIndex)
        $lines.Add(('Device.TopologyRef{0}="{1}"' -f $refIndex, $reference))
    }

    $parent = Split-Path -Parent $Path
    if ($parent) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    [IO.File]::WriteAllLines($Path, $lines, [Text.Encoding]::ASCII)
    return $Path
}
