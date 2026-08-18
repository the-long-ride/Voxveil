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

    if ($Targets.Count -eq 0) {
        throw 'Cannot generate a Voxveil Extension INF without an audio target.'
    }

    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add('[Version]')
    $lines.Add('Signature="$WINDOWS NT$"')
    $lines.Add('Class=Extension')
    $lines.Add('ClassGuid={e2f84ce7-8efa-411c-aa69-97454ca4cb57}')
    $lines.Add('ExtensionId={0D6F6D57-5BD4-4F45-9C40-6F70E34F0AA2}')
    $lines.Add('Provider=%ProviderName%')
    $lines.Add('DriverVer=08/18/2026,0.1.0.0')
    $lines.Add('PnpLockDown=1')
    $lines.Add('')
    $lines.Add('[Manufacturer]')
    $lines.Add('%MfgName%=DeviceExtensions,NTamd64.10.0...22000')
    $lines.Add('')
    $lines.Add('[DeviceExtensions.NTamd64.10.0...22000]')

    for ($index = 0; $index -lt $Targets.Count; $index++) {
        $lines.Add(('%Device{0}.Desc%=Device{0}_Install,%Device{0}.HardwareId%' -f $index))
    }

    for ($index = 0; $index -lt $Targets.Count; $index++) {
        $target = $Targets[$index]
        $refs = @($target.topologyRefs)
        if ($refs.Count -eq 0) {
            throw ("Target {0} has no topology interfaces." -f $target.instanceId)
        }

        $lines.Add('')
        $lines.Add(('[Device{0}_Install]' -f $index))
        $lines.Add('')
        $lines.Add(('[Device{0}_Install.Components]' -f $index))
        $lines.Add('AddComponent=VoxveilApo,,VoxveilApo_AddComponent')
        $lines.Add('')
        $lines.Add(('[Device{0}_Install.Interfaces]' -f $index))
        for ($refIndex = 0; $refIndex -lt $refs.Count; $refIndex++) {
            $lines.Add(('AddInterface=%KSCATEGORY_AUDIO%,%Device{0}.TopologyRef{1}%,VoxveilFx' -f $index, $refIndex))
            $lines.Add(('AddInterface=%KSCATEGORY_TOPOLOGY%,%Device{0}.TopologyRef{1}%,VoxveilFx' -f $index, $refIndex))
        }
    }

    $lines.Add('')
    $lines.Add('[VoxveilApo_AddComponent]')
    $lines.Add('ComponentIDs=VEN_VOXV&CID_APO')
    $lines.Add('Description="Voxveil Audio Processing Object"')
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
    $lines.Add('KSCATEGORY_AUDIO="{6994AD04-93EF-11D0-A3CC-00A0C9223196}"')
    $lines.Add('KSCATEGORY_TOPOLOGY="{DDA54A40-1E4C-11D1-A050-405705C10000}"')
    $lines.Add('PKEY_FX_Association="{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},0"')
    $lines.Add('PKEY_CompositeFX_EndpointEffectClsid="{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},15"')
    $lines.Add('PKEY_EFX_ProcessingModes_Supported_For_Streaming="{D3993A3F-99C2-4402-B5EC-A92A0367664B},7"')
    $lines.Add('AUDIO_SIGNALPROCESSINGMODE_DEFAULT="{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}"')
    $lines.Add('KSNODETYPE_ANY="{00000000-0000-0000-0000-000000000000}"')
    $lines.Add('VOXVEIL_APO_CLSID="{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}"')

    for ($index = 0; $index -lt $Targets.Count; $index++) {
        $target = $Targets[$index]
        $hardwareId = Assert-VoxveilInfString ([string]$target.hardwareId) ("Device{0} hardware ID" -f $index)
        $instanceId = Assert-VoxveilInfString ([string]$target.instanceId) ("Device{0} instance ID" -f $index)
        $lines.Add(('Device{0}.Desc="Voxveil audio extension for {1}"' -f $index, $instanceId))
        $lines.Add(('Device{0}.HardwareId="{1}"' -f $index, $hardwareId))
        $refs = @($target.topologyRefs)
        for ($refIndex = 0; $refIndex -lt $refs.Count; $refIndex++) {
            $reference = Assert-VoxveilInfString ([string]$refs[$refIndex]) ("Device{0} topology reference" -f $index)
            $lines.Add(('Device{0}.TopologyRef{1}="{2}"' -f $index, $refIndex, $reference))
        }
    }

    $parent = Split-Path -Parent $Path
    if ($parent) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    [IO.File]::WriteAllLines($Path, $lines, [Text.Encoding]::ASCII)
    return $Path
}
