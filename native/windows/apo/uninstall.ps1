param(
    [string]$PackageRoot = $PSScriptRoot
)

$ErrorActionPreference = 'Stop'
$Clsid = '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}'
$AudioKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$ComKey = "HKLM:\SOFTWARE\Classes\CLSID\$Clsid"
$ApoKey = "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$Clsid"
$StateRoot = Join-Path $env:ProgramData 'Voxveil'
$InstallRoot = Join-Path $env:ProgramFiles 'Voxveil\system-audio'
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
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', ('"{0}"' -f $PSCommandPath),
        '-PackageRoot', ('"{0}"' -f $PackageRoot)
    )
    $process = Start-Process -FilePath $powerShell -Verb RunAs -ArgumentList ($arguments -join ' ') -Wait -PassThru
    exit $process.ExitCode
}

function Set-Dword([string]$Path, [string]$Name, [uint32]$Value) {
    New-Item -Path $Path -Force | Out-Null
    New-ItemProperty -Path $Path -Name $Name -PropertyType DWord -Value $Value -Force | Out-Null
}

function Remove-DevelopmentApoRegistration {
    Remove-Item -LiteralPath $ApoKey -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ComKey -Recurse -Force -ErrorAction SilentlyContinue
}

function Restore-ProtectedAudio {
    if (-not (Test-Path -LiteralPath $ProtectedBackup -PathType Leaf)) {
        return
    }
    $backup = Get-Content -LiteralPath $ProtectedBackup -Raw | ConvertFrom-Json
    if ($backup.Exists) {
        Set-Dword $AudioKey 'DisableProtectedAudioDG' ([uint32]$backup.Value)
    } else {
        Remove-ItemProperty -LiteralPath $AudioKey -Name 'DisableProtectedAudioDG' -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $ProtectedBackup -Force -ErrorAction SilentlyContinue
}

function Find-FxHelper {
    foreach ($candidate in @(
        (Join-Path $InstallRoot 'VoxveilApoTarget.exe'),
        (Join-Path $PackageRoot 'VoxveilApoTarget.exe')
    )) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }
    throw 'VoxveilApoTarget.exe is unavailable; runtime audio-interface registration cannot be removed safely.'
}

function Remove-RuntimeFx {
    $helper = Find-FxHelper
    $output = (& $helper '--remove-fx' 2>&1 | Out-String).Trim()
    $exitCode = $LASTEXITCODE
    if ($output) {
        Write-Host $output
    }
    if ($exitCode -ne 0) {
        throw "Voxveil runtime FX removal failed with exit code $exitCode.`r`n$output"
    }
}

function Restart-AudioStack {
    try {
        Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction Stop
        Start-Service -Name 'Audiosrv' -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Audio services could not be restarted automatically: $($_.Exception.Message). Restart Windows to finish removing Voxveil system audio."
    }
}

if (-not (Test-Administrator)) {
    Invoke-ElevatedSelf
}

try {
    Remove-RuntimeFx
    Remove-DevelopmentApoRegistration
    Restore-ProtectedAudio
    Remove-Item -LiteralPath $Control -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Marker -Force -ErrorAction SilentlyContinue
    Restart-AudioStack
    Remove-Item -LiteralPath $InstallRoot -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host 'Voxveil runtime APO registration was removed and DisableProtectedAudioDG was restored.'
    exit 0
} catch {
    Write-Error ("Voxveil system-audio uninstall failed: {0}. Windows Test Mode was not changed." -f $_.Exception.Message)
    exit 1
}
