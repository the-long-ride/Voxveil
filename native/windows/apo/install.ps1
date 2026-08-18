param(
    [string]$PackageRoot = $PSScriptRoot,
    [string]$ResultPath = ''
)

$ErrorActionPreference = 'Stop'
$Clsid = '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}'
$ApoInterface = '{FD7F2B29-24D0-4B5C-B177-592C39F9CA10}'
$AudioKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$ComKey = "HKLM:\SOFTWARE\Classes\CLSID\$Clsid"
$ApoKey = "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$Clsid"
$StateRoot = Join-Path $env:ProgramData 'Voxveil'
$InstallRoot = Join-Path $env:ProgramFiles 'Voxveil\system-audio'
$ProtectedBackup = Join-Path $StateRoot 'protected-audio-backup.json'
$Marker = Join-Path $StateRoot 'apo-installed.json'
$Control = Join-Path $StateRoot 'apo-control.bin'
$script:ProtectedBackupCreated = $false
$script:ApoRegistered = $false
$script:FxRegistered = $false

function Write-InstallResult {
    param(
        [bool]$Success,
        [string]$Message,
        [string]$Details = ''
    )
    if (-not $ResultPath) {
        return
    }
    $parent = Split-Path -Parent $ResultPath
    if ($parent) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    [pscustomobject]@{
        success = $Success
        message = $Message
        details = $Details
    } | ConvertTo-Json -Compress | Set-Content -LiteralPath $ResultPath -Encoding UTF8
}

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
    if ($ResultPath) {
        $arguments += @('-ResultPath', ('"{0}"' -f $ResultPath))
    }
    $process = Start-Process -FilePath $powerShell -Verb RunAs -ArgumentList ($arguments -join ' ') -Wait -PassThru
    exit $process.ExitCode
}

function Get-RegistryValueInfo([string]$Path, [string]$Name) {
    try {
        $item = Get-ItemProperty -LiteralPath $Path -Name $Name -ErrorAction Stop
        return [pscustomobject]@{ Exists = $true; Value = $item.PSObject.Properties[$Name].Value }
    } catch {
        return [pscustomobject]@{ Exists = $false; Value = $null }
    }
}

function Set-Dword([string]$Path, [string]$Name, [uint32]$Value) {
    New-Item -Path $Path -Force | Out-Null
    New-ItemProperty -Path $Path -Name $Name -PropertyType DWord -Value $Value -Force | Out-Null
}

function Register-DevelopmentApo([string]$DllPath) {
    $com = New-Item -Path $ComKey -Force
    $com.SetValue('', 'Voxveil Endpoint Effect', [Microsoft.Win32.RegistryValueKind]::String)
    $inproc = New-Item -Path "$ComKey\InProcServer32" -Force
    $inproc.SetValue('', $DllPath, [Microsoft.Win32.RegistryValueKind]::String)
    $inproc.SetValue('ThreadingModel', 'Both', [Microsoft.Win32.RegistryValueKind]::String)

    $apo = New-Item -Path $ApoKey -Force
    $apo.SetValue('FriendlyName', 'Voxveil Endpoint Effect', [Microsoft.Win32.RegistryValueKind]::String)
    $apo.SetValue('Copyright', 'Voxveil contributors', [Microsoft.Win32.RegistryValueKind]::String)
    $apo.SetValue('MajorVersion', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $apo.SetValue('MinorVersion', 0, [Microsoft.Win32.RegistryValueKind]::DWord)
    # APO_FLAG_INPLACE (0x1) | APO_FLAG_DEFAULT (0xE).
    $apo.SetValue('Flags', 0x0000000f, [Microsoft.Win32.RegistryValueKind]::DWord)
    $apo.SetValue('MinInputConnections', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $apo.SetValue('MaxInputConnections', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $apo.SetValue('MinOutputConnections', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $apo.SetValue('MaxOutputConnections', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $apo.SetValue('MaxInstances', -1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $apo.SetValue('NumAPOInterfaces', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    $apo.SetValue('APOInterface0', $ApoInterface, [Microsoft.Win32.RegistryValueKind]::String)
}

function Remove-DevelopmentApoRegistration {
    Remove-Item -LiteralPath $ApoKey -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $ComKey -Recurse -Force -ErrorAction SilentlyContinue
}

function Test-ApoComServer([string]$DllPath) {
    $checker = Join-Path $PackageRoot 'VoxveilApoCheck.exe'
    if (-not (Test-Path -LiteralPath $checker -PathType Leaf)) {
        throw "Voxveil APO checker is missing: $checker"
    }
    $output = (& $checker $DllPath 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw ("Voxveil APO COM activation check failed with exit code {0}.`r`n{1}" -f $LASTEXITCODE, $output)
    }
    if ($output) {
        Write-Host $output
    }
}

function Enable-DevelopmentAudioGraph {
    if (-not (Test-Path -LiteralPath $ProtectedBackup -PathType Leaf)) {
        $old = Get-RegistryValueInfo $AudioKey 'DisableProtectedAudioDG'
        [pscustomobject]@{ Exists = $old.Exists; Value = $old.Value } |
            ConvertTo-Json | Set-Content -LiteralPath $ProtectedBackup -Encoding UTF8
        $script:ProtectedBackupCreated = $true
    }
    Set-Dword $AudioKey 'DisableProtectedAudioDG' 1
    Write-Warning 'Development build: DisableProtectedAudioDG=1 was enabled so the unsigned development APO can load. Uninstall restores the prior value.'
}

function Restore-ProtectedAudioIfCreated {
    if (-not $script:ProtectedBackupCreated -or -not (Test-Path -LiteralPath $ProtectedBackup -PathType Leaf)) {
        return
    }
    try {
        $backup = Get-Content -LiteralPath $ProtectedBackup -Raw | ConvertFrom-Json
        if ($backup.Exists) {
            Set-Dword $AudioKey 'DisableProtectedAudioDG' ([uint32]$backup.Value)
        } else {
            Remove-ItemProperty -LiteralPath $AudioKey -Name 'DisableProtectedAudioDG' -Force -ErrorAction SilentlyContinue
        }
        Remove-Item -LiteralPath $ProtectedBackup -Force -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Could not restore protected-audio development setting after failed install: $($_.Exception.Message)"
    }
}

function Invoke-FxHelper([string]$Helper, [string]$Command, [switch]$AllowFailure) {
    $output = (& $Helper $Command 2>&1 | Out-String).Trim()
    $exitCode = $LASTEXITCODE
    if ($output) {
        Write-Host $output
    }
    if ($exitCode -ne 0 -and -not $AllowFailure) {
        throw "Voxveil runtime FX registration failed with exit code $exitCode.`r`n$output"
    }
    return $exitCode
}

function Restart-AudioStack {
    try {
        Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction Stop
        Start-Service -Name 'Audiosrv' -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Audio services could not be restarted automatically: $($_.Exception.Message). Restart Windows before using Voxveil if processing is not available."
    }
}

if (-not (Test-Administrator)) {
    Invoke-ElevatedSelf
}

try {
    if ([Environment]::OSVersion.Version.Build -lt 22000) {
        throw 'This Voxveil APO development package currently requires Windows 11 build 22000 or newer.'
    }
    foreach ($required in @('VoxveilApo.dll', 'VoxveilApoCheck.exe', 'VoxveilApoTarget.exe', 'uninstall.ps1')) {
        $requiredPath = Join-Path $PackageRoot $required
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "Voxveil system-audio payload is missing: $required"
        }
    }

    New-Item -ItemType Directory -Path $StateRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null
    & icacls.exe $StateRoot /grant '*S-1-5-32-545:(OI)(CI)M' /T /C | Out-Null

    $targetDll = Join-Path $InstallRoot 'VoxveilApo.dll'
    $targetHelper = Join-Path $InstallRoot 'VoxveilApoTarget.exe'
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApo.dll') -Destination $targetDll -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApoTarget.exe') -Destination $targetHelper -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApoCheck.exe') -Destination (Join-Path $InstallRoot 'VoxveilApoCheck.exe') -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'uninstall.ps1') -Destination (Join-Path $InstallRoot 'uninstall.ps1') -Force

    Test-ApoComServer $targetDll
    Register-DevelopmentApo $targetDll
    $script:ApoRegistered = $true

    Invoke-FxHelper $targetHelper '--install-fx' | Out-Null
    $script:FxRegistered = $true

    Enable-DevelopmentAudioGraph
    [IO.File]::WriteAllBytes($Control, [byte[]](1, 0, 100))

    [pscustomobject]@{
        Version = 3
        Clsid = $Clsid
        DllPath = $targetDll
        Deployment = 'runtime-interface-development'
        InstalledAtUtc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Marker -Encoding UTF8

    Restart-AudioStack
    Write-InstallResult -Success $true -Message 'Voxveil Windows system audio component installed.'
    Write-Host 'Voxveil APO installed through runtime KSCATEGORY_AUDIO interface registration.'
    exit 0
} catch {
    $message = $_.Exception.Message
    if ($script:FxRegistered) {
        try {
            $helper = Join-Path $InstallRoot 'VoxveilApoTarget.exe'
            if (Test-Path -LiteralPath $helper -PathType Leaf) {
                Invoke-FxHelper $helper '--remove-fx' -AllowFailure | Out-Null
            }
        } catch {
            Write-Warning "Could not roll back Voxveil FX registration: $($_.Exception.Message)"
        }
    }
    if ($script:ApoRegistered) {
        Remove-DevelopmentApoRegistration
    }
    Restore-ProtectedAudioIfCreated
    $details = "$message`r`nThis development path does not enable Windows Test Mode and does not install a driver package."
    Write-InstallResult -Success $false -Message 'Voxveil Windows system-audio installation failed.' -Details $details
    Write-Error $details
    exit 1
}
