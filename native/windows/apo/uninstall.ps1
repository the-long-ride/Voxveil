$ErrorActionPreference = 'Stop'
$Clsid = '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}'
$SingleEfx = '{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},7'
$CompositeEfx = '{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D},15'
$AudioKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$RenderRoot = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render'
$StateRoot = Join-Path $env:ProgramData 'Voxveil'
$InstallRoot = Join-Path $env:ProgramFiles 'Voxveil\system-audio'
$EndpointBackup = Join-Path $StateRoot 'endpoint-backup.json'
$ProtectedBackup = Join-Path $StateRoot 'protected-audio-backup.json'

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Invoke-ElevatedSelf {
    $powerShell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', ('"{0}"' -f $PSCommandPath)) -join ' '
    $process = Start-Process -FilePath $powerShell -Verb RunAs -ArgumentList $arguments -Wait -PassThru
    exit $process.ExitCode
}

function Restore-Endpoint($backup) {
    $fx = Join-Path (Join-Path $RenderRoot $backup.Endpoint) 'FxProperties'
    if (-not (Test-Path -LiteralPath $fx)) {
        return
    }

    if ($backup.CompositeExists) {
        New-ItemProperty -Path $fx -Name $CompositeEfx -PropertyType MultiString -Value @($backup.CompositeValue) -Force | Out-Null
    } else {
        Remove-ItemProperty -Path $fx -Name $CompositeEfx -Force -ErrorAction SilentlyContinue
    }

    if ($backup.SingleExists) {
        New-ItemProperty -Path $fx -Name $SingleEfx -PropertyType String -Value ([string]$backup.SingleValue) -Force | Out-Null
    } else {
        Remove-ItemProperty -Path $fx -Name $SingleEfx -Force -ErrorAction SilentlyContinue
    }
}

function Restore-ProtectedAudio {
    if (-not (Test-Path -LiteralPath $ProtectedBackup)) {
        return
    }
    $backup = Get-Content -LiteralPath $ProtectedBackup -Raw | ConvertFrom-Json
    if ($backup.Exists) {
        New-Item -Path $AudioKey -Force | Out-Null
        New-ItemProperty -Path $AudioKey -Name 'DisableProtectedAudioDG' -PropertyType DWord -Value ([uint32]$backup.Value) -Force | Out-Null
    } else {
        Remove-ItemProperty -Path $AudioKey -Name 'DisableProtectedAudioDG' -Force -ErrorAction SilentlyContinue
    }
}

function Restart-AudioServices {
    try {
        Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction Stop
        Start-Service -Name 'Audiosrv' -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Audio services could not be restarted automatically: $($_.Exception.Message). Reboot Windows to complete removal."
    }
}

if (-not (Test-Administrator)) {
    Invoke-ElevatedSelf
}

if (Test-Path -LiteralPath $EndpointBackup) {
    $backups = @(Get-Content -LiteralPath $EndpointBackup -Raw | ConvertFrom-Json)
    foreach ($backup in $backups) {
        Restore-Endpoint $backup
    }
}

Remove-Item -LiteralPath "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$Clsid" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "HKLM:\SOFTWARE\Classes\CLSID\$Clsid" -Recurse -Force -ErrorAction SilentlyContinue
Restore-ProtectedAudio
Restart-AudioServices

Remove-Item -LiteralPath $InstallRoot -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $StateRoot 'apo-installed.json') -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $StateRoot 'apo-control.bin') -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $EndpointBackup -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $ProtectedBackup -Force -ErrorAction SilentlyContinue

Write-Host 'Voxveil system audio component removed and previous endpoint effects restored.'
