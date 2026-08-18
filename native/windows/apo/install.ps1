param(
    [string]$PackageRoot = $PSScriptRoot,
    [string]$ResultPath = ''
)

$ErrorActionPreference = 'Stop'
$Clsid = '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}'
$AudioKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$StateRoot = Join-Path $env:ProgramData 'Voxveil'
$InstallRoot = Join-Path $env:ProgramFiles 'Voxveil\system-audio'
$ProtectedBackup = Join-Path $StateRoot 'protected-audio-backup.json'
$Marker = Join-Path $StateRoot 'apo-installed.json'
$Control = Join-Path $StateRoot 'apo-control.bin'
$PnpUtil = Join-Path $env:SystemRoot 'System32\pnputil.exe'
$GeneratedExtension = Join-Path $PackageRoot 'VoxveilAudioExtension.inf'
$script:PnpLog = New-Object System.Collections.Generic.List[string]
$script:InstalledPackages = New-Object System.Collections.Generic.List[string]
$script:Targets = @()
$script:ProtectedBackupCreated = $false

. (Join-Path $PackageRoot 'targets.ps1')
. (Join-Path $PackageRoot 'extension.ps1')

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

function Remove-LegacyGlobalRegistration {
    Remove-Item -LiteralPath "HKLM:\SOFTWARE\Classes\CLSID\$Clsid" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath "HKLM:\SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$Clsid" -Recurse -Force -ErrorAction SilentlyContinue
}

function Test-ApoComServer([string]$DllPath) {
    $checker = Join-Path $PackageRoot 'VoxveilApoCheck.exe'
    if (-not (Test-Path -LiteralPath $checker -PathType Leaf)) {
        throw "Voxveil APO checker is missing: $checker"
    }
    $output = & $checker $DllPath 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw ("Voxveil APO COM activation check failed with exit code {0}.`r`n{1}" -f $LASTEXITCODE, $output.Trim())
    }
}

function Add-PublishedPackages([string]$Text) {
    foreach ($match in [regex]::Matches($Text, '(?i)\boem\d+\.inf\b')) {
        if (-not $script:InstalledPackages.Contains($match.Value.ToLowerInvariant())) {
            $script:InstalledPackages.Add($match.Value.ToLowerInvariant())
        }
    }
}

function Invoke-PnpUtil {
    param(
        [string]$Label,
        [string[]]$Arguments,
        [switch]$AllowFailure
    )
    $output = (& $PnpUtil @Arguments 2>&1 | Out-String).Trim()
    $exitCode = $LASTEXITCODE
    $entry = "[$Label] pnputil $($Arguments -join ' ')`r`nExit code: $exitCode`r`n$output"
    $script:PnpLog.Add($entry)
    Add-PublishedPackages $output
    if ($exitCode -ne 0 -and -not $AllowFailure) {
        throw ("$Label failed with exit code $exitCode.`r`n$output")
    }
    return [pscustomobject]@{ ExitCode = $exitCode; Output = $output }
}

function Enable-DevelopmentAudioGraph {
    if (-not (Test-Path -LiteralPath $ProtectedBackup -PathType Leaf)) {
        $old = Get-RegistryValueInfo $AudioKey 'DisableProtectedAudioDG'
        [pscustomobject]@{ Exists = $old.Exists; Value = $old.Value } |
            ConvertTo-Json | Set-Content -LiteralPath $ProtectedBackup -Encoding UTF8
        $script:ProtectedBackupCreated = $true
    }
    Set-Dword $AudioKey 'DisableProtectedAudioDG' 1
    Write-Warning 'Development build: DisableProtectedAudioDG=1 was enabled. The Voxveil uninstaller restores its prior value.'
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

function Rollback-PnpPackages {
    $packages = @($script:InstalledPackages | Select-Object -Unique)
    for ($index = $packages.Count - 1; $index -ge 0; $index--) {
        $package = [string]$packages[$index]
        if ([string]::IsNullOrWhiteSpace($package)) {
            continue
        }
        try {
            Invoke-PnpUtil -Label "Rollback $package" -Arguments @('/delete-driver', $package, '/uninstall', '/force') -AllowFailure | Out-Null
        } catch {
            Write-Warning "Could not roll back $package`: $($_.Exception.Message)"
        }
    }
}

function Get-SetupApiTail {
    $logPath = Join-Path $env:WINDIR 'INF\setupapi.dev.log'
    if (-not (Test-Path -LiteralPath $logPath -PathType Leaf)) {
        return 'setupapi.dev.log was not found.'
    }
    return (Get-Content -LiteralPath $logPath -Tail 180 -ErrorAction SilentlyContinue | Out-String).Trim()
}

function Restart-AudioStack {
    foreach ($target in $script:Targets) {
        try {
            Invoke-PnpUtil -Label ("Restart audio device {0}" -f $target.instanceId) `
                -Arguments @('/restart-device', [string]$target.instanceId) -AllowFailure | Out-Null
        } catch {
            Write-Warning $_.Exception.Message
        }
    }
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
        throw 'This Voxveil APO package currently requires Windows 11 build 22000 or newer.'
    }
    foreach ($required in @('VoxveilApo.dll', 'VoxveilApoCheck.exe', 'VoxveilApoTarget.exe', 'VoxveilApo.inf', 'targets.ps1', 'extension.ps1')) {
        $requiredPath = Join-Path $PackageRoot $required
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "Voxveil system-audio payload is missing: $required"
        }
    }

    New-Item -ItemType Directory -Path $StateRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null
    & icacls.exe $StateRoot /grant '*S-1-5-32-545:(OI)(CI)M' /T /C | Out-Null

    $targetDll = Join-Path $InstallRoot 'VoxveilApo.dll'
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApo.dll') -Destination $targetDll -Force
    Test-ApoComServer $targetDll
    Remove-LegacyGlobalRegistration

    $script:Targets = @(Get-VoxveilAudioTargets -PackageRoot $PackageRoot)
    New-VoxveilExtensionInf -Targets $script:Targets -Path $GeneratedExtension | Out-Null
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApo.inf') -Destination (Join-Path $InstallRoot 'VoxveilApo.inf') -Force
    Copy-Item -LiteralPath $GeneratedExtension -Destination (Join-Path $InstallRoot 'VoxveilAudioExtension.inf') -Force

    Invoke-PnpUtil -Label 'Stage Voxveil APO package' -Arguments @('/add-driver', (Join-Path $PackageRoot 'VoxveilApo.inf')) | Out-Null
    Invoke-PnpUtil -Label 'Install Voxveil audio extension' -Arguments @('/add-driver', $GeneratedExtension, '/install') | Out-Null
    Invoke-PnpUtil -Label 'Scan for Voxveil APO component' -Arguments @('/scan-devices') | Out-Null
    Invoke-PnpUtil -Label 'Bind Voxveil APO component' -Arguments @('/add-driver', (Join-Path $PackageRoot 'VoxveilApo.inf'), '/install') | Out-Null

    Enable-DevelopmentAudioGraph
    [IO.File]::WriteAllBytes($Control, [byte[]](1, 0, 100))

    [pscustomobject]@{
        Version = 2
        Clsid = $Clsid
        DllPath = $targetDll
        Deployment = 'componentized-inf'
        OemInfs = @($script:InstalledPackages | Select-Object -Unique)
        Targets = $script:Targets
        InstalledAtUtc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Marker -Encoding UTF8

    Restart-AudioStack
    $message = "Voxveil system audio component installed through Windows PnP on $($script:Targets.Count) render device(s)."
    $details = ($script:PnpLog -join "`r`n`r`n")
    Write-InstallResult -Success $true -Message $message -Details $details
    Write-Host $message
    Write-Host 'Restart Voxveil, then enable Processing.'
    exit 0
} catch {
    $failure = $_
    Rollback-PnpPackages
    Restore-ProtectedAudioIfCreated
    Remove-Item -LiteralPath $Marker -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Control -Force -ErrorAction SilentlyContinue

    $lines = @(
        "Message: $($failure.Exception.Message)",
        "Category: $($failure.CategoryInfo)",
        "FullyQualifiedErrorId: $($failure.FullyQualifiedErrorId)"
    )
    if ($failure.InvocationInfo.PositionMessage) {
        $lines += $failure.InvocationInfo.PositionMessage
    }
    if ($failure.ScriptStackTrace) {
        $lines += "ScriptStackTrace: $($failure.ScriptStackTrace)"
    }
    if ($script:Targets.Count -gt 0) {
        $lines += "DiscoveredTargets:`r`n$($script:Targets | ConvertTo-Json -Depth 6)"
    }
    if (Test-Path -LiteralPath $GeneratedExtension -PathType Leaf) {
        $lines += "GeneratedExtensionInf:`r`n$(Get-Content -LiteralPath $GeneratedExtension -Raw)"
    }
    if ($script:PnpLog.Count -gt 0) {
        $lines += "PnPUtil:`r`n$($script:PnpLog -join "`r`n`r`n")"
    }
    $lines += "SetupApiDevLogTail:`r`n$(Get-SetupApiTail)"
    $details = ($lines -join "`r`n`r`n").Trim()
    Write-InstallResult -Success $false -Message $failure.Exception.Message -Details $details
    Write-Error $details
    exit 1
}
