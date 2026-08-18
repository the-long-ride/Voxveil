$ErrorActionPreference = 'Stop'
$Clsid = '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}'
$AudioKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$StateRoot = Join-Path $env:ProgramData 'Voxveil'
$InstallRoot = Join-Path $env:ProgramFiles 'Voxveil\system-audio'
$Marker = Join-Path $StateRoot 'apo-installed.json'
$ProtectedBackup = Join-Path $StateRoot 'protected-audio-backup.json'
$PnpUtil = Join-Path $env:SystemRoot 'System32\pnputil.exe'

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

function Invoke-PnpDelete([string]$Package) {
    if ([string]::IsNullOrWhiteSpace($Package) -or $Package -notmatch '^(?i)oem\d+\.inf$') {
        return
    }
    $output = (& $PnpUtil /delete-driver $Package /uninstall /force 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Could not remove PnP package $Package (exit $LASTEXITCODE): $output"
    } elseif ($output) {
        Write-Host $output
    }
}

function Find-VoxveilDriverPackages {
    try {
        $drivers = @(Get-WindowsDriver -Online -All -ErrorAction Stop | Where-Object {
            $_.ProviderName -eq 'Voxveil' -or
            ([string]$_.OriginalFileName -match '(?i)Voxveil(?:Apo|AudioExtension)\.inf$')
        })
        return @($drivers | ForEach-Object { [string]$_.Driver } | Where-Object { $_ -match '^(?i)oem\d+\.inf$' })
    } catch {
        Write-Warning "Could not enumerate Voxveil driver-store packages through DISM: $($_.Exception.Message)"
        return @()
    }
}

function Restore-ProtectedAudio {
    if (-not (Test-Path -LiteralPath $ProtectedBackup -PathType Leaf)) {
        return
    }
    $backup = Get-Content -LiteralPath $ProtectedBackup -Raw | ConvertFrom-Json
    if ($backup.Exists) {
        New-Item -Path $AudioKey -Force | Out-Null
        New-ItemProperty -Path $AudioKey -Name 'DisableProtectedAudioDG' -PropertyType DWord -Value ([uint32]$backup.Value) -Force | Out-Null
    } else {
        Remove-ItemProperty -LiteralPath $AudioKey -Name 'DisableProtectedAudioDG' -Force -ErrorAction SilentlyContinue
    }
}

function Remove-LegacyGlobalRegistration {
    Remove-Item -LiteralPath "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$Clsid" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath "HKLM:\SOFTWARE\Classes\CLSID\$Clsid" -Recurse -Force -ErrorAction SilentlyContinue
}

function Restart-AudioStack($Targets) {
    foreach ($target in @($Targets)) {
        $instanceId = [string]$target.instanceId
        if ([string]::IsNullOrWhiteSpace($instanceId)) {
            continue
        }
        $output = (& $PnpUtil /restart-device $instanceId 2>&1 | Out-String).Trim()
        if ($LASTEXITCODE -ne 0 -and $output) {
            Write-Warning $output
        }
    }
    try {
        Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction Stop
        Start-Service -Name 'Audiosrv' -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Audio services could not be restarted automatically: $($_.Exception.Message). Restart Windows to complete removal."
    }
}

if (-not (Test-Administrator)) {
    Invoke-ElevatedSelf
}

$markerData = $null
if (Test-Path -LiteralPath $Marker -PathType Leaf) {
    try {
        $markerData = Get-Content -LiteralPath $Marker -Raw | ConvertFrom-Json
    } catch {
        Write-Warning "Could not parse Voxveil installation marker: $($_.Exception.Message)"
    }
}

$packages = @()
if ($markerData) {
    $packages += @($markerData.OemInfs)
}
$packages += @(Find-VoxveilDriverPackages)
$packages = @($packages | ForEach-Object { [string]$_ } | Where-Object { $_ } | Select-Object -Unique)
foreach ($package in $packages) {
    Invoke-PnpDelete $package
}

Remove-LegacyGlobalRegistration
Restore-ProtectedAudio
Restart-AudioStack $(if ($markerData) { $markerData.Targets } else { @() })

Remove-Item -LiteralPath $InstallRoot -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $Marker -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $StateRoot 'apo-control.bin') -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $StateRoot 'endpoint-backup.json') -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $ProtectedBackup -Force -ErrorAction SilentlyContinue

Write-Host 'Voxveil system audio component and its PnP packages were removed.'
