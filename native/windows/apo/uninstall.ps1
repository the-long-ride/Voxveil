param(
    [string]$PackageRoot = $PSScriptRoot
)

$ErrorActionPreference = 'Stop'
$Clsid = '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}'
$AudioKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$LegacyComKey = "HKLM:\SOFTWARE\Classes\CLSID\$Clsid"
$LegacyApoKey = "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$Clsid"
$StateRoot = Join-Path $env:ProgramData 'Voxveil'
$InstallRoot = Join-Path $env:ProgramFiles 'Voxveil\system-audio'
$ProtectedBackup = Join-Path $StateRoot 'protected-audio-backup.json'
$Marker = Join-Path $StateRoot 'apo-installed.json'
$Control = Join-Path $StateRoot 'apo-control.bin'
$Runtime = Join-Path $StateRoot 'apo-runtime.bin'
$CertificatePath = Join-Path $StateRoot 'voxveil-apo-development.cer'

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

function Restore-ProtectedAudio {
    if (-not (Test-Path -LiteralPath $ProtectedBackup -PathType Leaf)) { return }
    $backup = Get-Content -LiteralPath $ProtectedBackup -Raw | ConvertFrom-Json
    if ($backup.Exists) {
        Set-Dword $AudioKey 'DisableProtectedAudioDG' ([uint32]$backup.Value)
    } else {
        Remove-ItemProperty -LiteralPath $AudioKey -Name 'DisableProtectedAudioDG' -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $ProtectedBackup -Force -ErrorAction SilentlyContinue
}

function Invoke-PnpUtil([string[]]$Arguments, [switch]$AllowFailure) {
    $output = (& pnputil.exe @Arguments 2>&1 | Out-String).Trim()
    $exitCode = $LASTEXITCODE
    if ($output) { Write-Host $output }
    if ($exitCode -ne 0 -and -not $AllowFailure) {
        throw ("pnputil {0} failed with exit code {1}.`r`n{2}" -f ($Arguments -join ' '), $exitCode, $output)
    }
    return $exitCode
}

function Find-TargetHelper {
    foreach ($candidate in @(
        (Join-Path $InstallRoot 'VoxveilApoTarget.exe'),
        (Join-Path $PackageRoot 'VoxveilApoTarget.exe')
    )) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    return $null
}

function Cleanup-LegacyRuntimeRegistration {
    $helper = Find-TargetHelper
    if (-not $helper) { return }
    $output = (& $helper '--cleanup-runtime' 2>&1 | Out-String).Trim()
    if ($output) { Write-Host $output }
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Legacy runtime FX cleanup failed: $output"
    }
}

function Remove-VoxveilDriverPackages {
    Import-Module Dism -ErrorAction Stop
    $drivers = @(Get-WindowsDriver -Online | Where-Object { $_.ProviderName -eq 'Voxveil' })
    foreach ($driver in $drivers) {
        if ($driver.Driver) {
            Invoke-PnpUtil @('/delete-driver', [string]$driver.Driver, '/uninstall', '/force') -AllowFailure | Out-Null
        }
    }
}

function Remove-DevelopmentTrust([string]$Thumbprint) {
    if ([string]::IsNullOrWhiteSpace($Thumbprint)) { return }
    foreach ($store in @('My', 'Root', 'TrustedPublisher')) {
        Remove-Item -LiteralPath ("Cert:\LocalMachine\{0}\{1}" -f $store, $Thumbprint) -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $CertificatePath -Force -ErrorAction SilentlyContinue
}

function Restart-AudioTarget([string]$InstanceId) {
    if (-not [string]::IsNullOrWhiteSpace($InstanceId)) {
        Invoke-PnpUtil @('/restart-device', $InstanceId) -AllowFailure | Out-Null
    }
    try {
        Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction Stop
        Start-Service -Name 'Audiosrv' -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Audio services could not be restarted automatically: $($_.Exception.Message). Restart Windows to finish removing Voxveil system audio."
    }
}

if (-not (Test-Administrator)) { Invoke-ElevatedSelf }

try {
    $instanceId = ''
    $thumbprint = ''
    if (Test-Path -LiteralPath $Marker -PathType Leaf) {
        $state = Get-Content -LiteralPath $Marker -Raw | ConvertFrom-Json
        $instanceId = [string]$state.TargetInstanceId
        $thumbprint = [string]$state.SigningThumbprint
    }

    Cleanup-LegacyRuntimeRegistration
    Remove-VoxveilDriverPackages
    Remove-Item -LiteralPath $LegacyApoKey -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $LegacyComKey -Recurse -Force -ErrorAction SilentlyContinue
    Restore-ProtectedAudio
    Remove-DevelopmentTrust $thumbprint
    Remove-Item -LiteralPath $Control -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Runtime -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Marker -Force -ErrorAction SilentlyContinue
    Restart-AudioTarget $instanceId
    Remove-Item -LiteralPath $InstallRoot -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host 'Voxveil componentized APO packages were removed and development trust/protected-audio state were restored.'
    exit 0
} catch {
    Write-Error ("Voxveil system-audio uninstall failed: {0}. Windows Test Mode was not changed." -f $_.Exception.Message)
    exit 1
}
