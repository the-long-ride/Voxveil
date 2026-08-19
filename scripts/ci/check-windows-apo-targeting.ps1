$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
. (Join-Path $root 'native\windows\apo\extension.ps1')

$temp = Join-Path $env:TEMP ("voxveil-extension-fixture-{0}" -f [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp -Force | Out-Null
try {
    $output = Join-Path $temp 'VoxveilExtension.inf'
    $targets = @(
        [pscustomobject]@{
            instanceId = 'USB\VID_046D&PID_0A4D\FIXTURE'
            hardwareId = 'USB\VID_046D&PID_0A4D'
            topologyRefs = @('Topology', 'SpeakerTopology')
        }
    )

    New-VoxveilExtensionInf -Targets $targets -Path $output | Out-Null
    if (-not (Test-Path -LiteralPath $output -PathType Leaf)) {
        throw 'Extension INF generator did not create its output file.'
    }

    $text = Get-Content -LiteralPath $output -Raw
    $required = @(
        'Class=Extension',
        'CatalogFile=VoxveilExtension.cat',
        'USB\VID_046D&PID_0A4D',
        'AddComponent=VoxveilApo,,VoxveilApo_AddComponent',
        'ComponentIDs=VEN_VOXV&CID_APO',
        'AddInterface=%KSCATEGORY_AUDIO%,%Device.TopologyRef0%,VoxveilFx',
        'AddInterface=%KSCATEGORY_TOPOLOGY%,%Device.TopologyRef0%,VoxveilFx',
        'AddInterface=%KSCATEGORY_AUDIO%,%Device.TopologyRef1%,VoxveilFx',
        'KSCATEGORY_AUDIO="{6994AD04-93EF-11D0-A3CC-00A0C9223196}"',
        'KSCATEGORY_TOPOLOGY="{DDA54A40-1E4C-11D1-A050-405705C10000}"',
        'Device.TopologyRef0="Topology"',
        'Device.TopologyRef1="SpeakerTopology"',
        'PKEY_CompositeFX_EndpointEffectClsid',
        '0x00010008',
        '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}',
        'PKEY_EFX_ProcessingModes_Supported_For_Streaming',
        '{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}'
    )
    foreach ($needle in $required) {
        if (-not $text.Contains($needle)) {
            throw "Generated Extension INF is missing required content: $needle"
        }
    }
    if ($text -match 'MMDevices\\Audio\\Render') {
        throw 'Generated Extension INF must not reference the protected MMDevices endpoint store.'
    }

    $badTarget = @(
        [pscustomobject]@{
            instanceId = 'fixture'
            hardwareId = "BAD`nID"
            topologyRefs = @('Topology')
        }
    )
    $rejected = $false
    try {
        New-VoxveilExtensionInf -Targets $badTarget -Path (Join-Path $temp 'bad.inf') | Out-Null
    } catch {
        $rejected = $true
    }
    if (-not $rejected) {
        throw 'Extension INF generator accepted an unsafe hardware ID.'
    }

    $multipleTargetsRejected = $false
    try {
        New-VoxveilExtensionInf -Targets @($targets[0], $targets[0]) -Path (Join-Path $temp 'multi.inf') | Out-Null
    } catch {
        $multipleTargetsRejected = $true
    }
    if (-not $multipleTargetsRejected) {
        throw 'Extension INF generator accepted more than one render target.'
    }

    Write-Host 'Voxveil Extension INF fixture verification passed.'
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
