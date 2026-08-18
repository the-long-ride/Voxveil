param(
    [string]$PackageRoot = $PSScriptRoot
)

$ErrorActionPreference = 'Stop'
$Clsid = '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}'
$ApoInterface = '{FD7F2B29-24D0-4B5C-B177-592C39F9CA10}'
$SingleEfx = '{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},7'
$CompositeEfx = '{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},15'
$AudioKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$RenderRoot = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render'
$StateRoot = Join-Path $env:ProgramData 'Voxveil'
$InstallRoot = Join-Path $env:ProgramFiles 'Voxveil\system-audio'
$EndpointBackup = Join-Path $StateRoot 'endpoint-backup.json'
$ProtectedBackup = Join-Path $StateRoot 'protected-audio-backup.json'
$Marker = Join-Path $StateRoot 'apo-installed.json'
$Control = Join-Path $StateRoot 'apo-control.bin'

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Invoke-ElevatedSelf {
    $powerShell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $arguments = @(
        '-NoProfile',
        '-ExecutionPolicy', 'Bypass',
        '-File', ('"{0}"' -f $PSCommandPath),
        '-PackageRoot', ('"{0}"' -f $PackageRoot)
    ) -join ' '
    $process = Start-Process -FilePath $powerShell -Verb RunAs -ArgumentList $arguments -Wait -PassThru
    exit $process.ExitCode
}

function Get-RegistryValueInfo([string]$Path, [string]$Name) {
    try {
        $item = Get-ItemProperty -LiteralPath $Path -Name $Name -ErrorAction Stop
        return [pscustomobject]@{
            Exists = $true
            Value = $item.PSObject.Properties[$Name].Value
        }
    } catch {
        return [pscustomobject]@{ Exists = $false; Value = $null }
    }
}

function Set-Dword([string]$Path, [string]$Name, [uint32]$Value) {
    New-Item -Path $Path -Force | Out-Null
    New-ItemProperty -Path $Path -Name $Name -PropertyType DWord -Value $Value -Force | Out-Null
}

function Register-Apo([string]$DllPath) {
    $com = "HKLM:\SOFTWARE\Classes\CLSID\$Clsid"
    $inproc = Join-Path $com 'InprocServer32'
    New-Item -Path $inproc -Force | Out-Null
    Set-Item -Path $com -Value 'Voxveil Endpoint Effect'
    Set-Item -Path $inproc -Value $DllPath
    New-ItemProperty -Path $inproc -Name 'ThreadingModel' -Value 'Both' -PropertyType String -Force | Out-Null

    $apo = "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$Clsid"
    New-Item -Path $apo -Force | Out-Null
    New-ItemProperty -Path $apo -Name 'FriendlyName' -Value 'Voxveil Endpoint Effect' -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $apo -Name 'Copyright' -Value 'Voxveil contributors' -PropertyType String -Force | Out-Null
    Set-Dword $apo 'MajorVersion' 1
    Set-Dword $apo 'MinorVersion' 0
    Set-Dword $apo 'Flags' 15
    Set-Dword $apo 'MinInputConnections' 1
    Set-Dword $apo 'MaxInputConnections' 1
    Set-Dword $apo 'MinOutputConnections' 1
    Set-Dword $apo 'MaxOutputConnections' 1
    Set-Dword $apo 'MaxInstances' ([uint32]::MaxValue)
    Set-Dword $apo 'NumAPOInterfaces' 1
    New-ItemProperty -Path $apo -Name 'APOInterface0' -Value $ApoInterface -PropertyType String -Force | Out-Null
}

function Attach-Endpoints {
    $backups = @()
    if (-not (Test-Path $RenderRoot)) {
        throw 'Windows render endpoint registry root was not found.'
    }

    foreach ($endpoint in Get-ChildItem -LiteralPath $RenderRoot -ErrorAction Stop) {
        $fx = Join-Path $endpoint.PSPath 'FxProperties'
        New-Item -Path $fx -Force | Out-Null
        $single = Get-RegistryValueInfo $fx $SingleEfx
        $composite = Get-RegistryValueInfo $fx $CompositeEfx
        $backups += [pscustomobject]@{
            Endpoint = $endpoint.PSChildName
            SingleExists = $single.Exists
            SingleValue = $single.Value
            CompositeExists = $composite.Exists
            CompositeValue = @($composite.Value)
        }

        $effects = @()
        if ($composite.Exists) {
            $effects += @($composite.Value)
        } elseif ($single.Exists -and $single.Value) {
            $effects += [string]$single.Value
        }
        $effects += $Clsid
        $effects = @($effects | Where-Object { $_ } | Select-Object -Unique)
        New-ItemProperty -Path $fx -Name $CompositeEfx -PropertyType MultiString -Value $effects -Force | Out-Null
        if ($single.Exists) {
            Remove-ItemProperty -Path $fx -Name $SingleEfx -Force -ErrorAction SilentlyContinue
        }
    }

    $backups | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EndpointBackup -Encoding UTF8
    return $backups.Count
}

function Enable-DevelopmentAudioGraph {
    $old = Get-RegistryValueInfo $AudioKey 'DisableProtectedAudioDG'
    [pscustomobject]@{ Exists = $old.Exists; Value = $old.Value } |
        ConvertTo-Json | Set-Content -LiteralPath $ProtectedBackup -Encoding UTF8
    Set-Dword $AudioKey 'DisableProtectedAudioDG' 1
    Write-Warning 'Development build: DisableProtectedAudioDG=1 was enabled. uninstall.ps1 restores its prior value.'
}

function Restart-AudioServices {
    try {
        Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction Stop
        Start-Service -Name 'Audiosrv' -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Audio services could not be restarted automatically: $($_.Exception.Message). Reboot Windows before using Voxveil."
    }
}

if (-not (Test-Administrator)) {
    Invoke-ElevatedSelf
}

$sourceDll = Join-Path $PackageRoot 'VoxveilApo.dll'
if (-not (Test-Path -LiteralPath $sourceDll -PathType Leaf)) {
    throw "VoxveilApo.dll was not found in package root: $PackageRoot"
}

New-Item -ItemType Directory -Path $StateRoot -Force | Out-Null
New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null
& icacls.exe $StateRoot /grant '*S-1-5-32-545:(OI)(CI)M' /T /C | Out-Null

$targetDll = Join-Path $InstallRoot 'VoxveilApo.dll'
Copy-Item -LiteralPath $sourceDll -Destination $targetDll -Force
Register-Apo $targetDll
$endpointCount = Attach-Endpoints
Enable-DevelopmentAudioGraph
[IO.File]::WriteAllBytes($Control, [byte[]](1, 0, 100))

[pscustomobject]@{
    Version = 1
    Clsid = $Clsid
    DllPath = $targetDll
    EndpointCount = $endpointCount
    InstalledAtUtc = [DateTime]::UtcNow.ToString('o')
} | ConvertTo-Json | Set-Content -LiteralPath $Marker -Encoding UTF8

Restart-AudioServices
Write-Host "Voxveil system audio component installed on $endpointCount render endpoint(s)."
Write-Host 'Restart Voxveil, then enable Processing.'
