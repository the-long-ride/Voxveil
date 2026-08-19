param(
    [string]$PackageRoot = $PSScriptRoot,
    [string]$ResultPath = ''
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
$script:ProtectedBackupCreated = $false
$script:SigningThumbprint = ''
$script:PackagesTouched = $false

. (Join-Path $PackageRoot 'targets.ps1')
. (Join-Path $PackageRoot 'extension.ps1')
. (Join-Path $PackageRoot 'catalog.ps1')

function Write-InstallResult {
    param([bool]$Success, [string]$Message, [string]$Details = '')
    if (-not $ResultPath) { return }
    $parent = Split-Path -Parent $ResultPath
    if ($parent) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
    [pscustomobject]@{ success = $Success; message = $Message; details = $Details } |
        ConvertTo-Json -Compress | Set-Content -LiteralPath $ResultPath -Encoding UTF8
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
    if ($ResultPath) { $arguments += @('-ResultPath', ('"{0}"' -f $ResultPath)) }
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
    Remove-Item -LiteralPath $LegacyApoKey -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $LegacyComKey -Recurse -Force -ErrorAction SilentlyContinue
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
    if ($output) { Write-Host $output }
}

function Enable-DevelopmentAudioGraph {
    if (-not (Test-Path -LiteralPath $ProtectedBackup -PathType Leaf)) {
        $old = Get-RegistryValueInfo $AudioKey 'DisableProtectedAudioDG'
        [pscustomobject]@{ Exists = $old.Exists; Value = $old.Value } |
            ConvertTo-Json | Set-Content -LiteralPath $ProtectedBackup -Encoding UTF8
        $script:ProtectedBackupCreated = $true
    }
    Set-Dword $AudioKey 'DisableProtectedAudioDG' 1
    Write-Warning 'Development build: DisableProtectedAudioDG=1 is enabled for the unsigned APO DLL. Windows TESTSIGNING is not changed.'
}

function Restore-ProtectedAudioIfCreated {
    if (-not $script:ProtectedBackupCreated -or -not (Test-Path -LiteralPath $ProtectedBackup -PathType Leaf)) { return }
    try {
        $backup = Get-Content -LiteralPath $ProtectedBackup -Raw | ConvertFrom-Json
        if ($backup.Exists) {
            Set-Dword $AudioKey 'DisableProtectedAudioDG' ([uint32]$backup.Value)
        } else {
            Remove-ItemProperty -LiteralPath $AudioKey -Name 'DisableProtectedAudioDG' -Force -ErrorAction SilentlyContinue
        }
        Remove-Item -LiteralPath $ProtectedBackup -Force -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Could not restore protected-audio setting after failed install: $($_.Exception.Message)"
    }
}

function Invoke-TargetHelper([string]$Command, [switch]$AllowFailure) {
    $helper = Join-Path $PackageRoot 'VoxveilApoTarget.exe'
    $output = (& $helper $Command 2>&1 | Out-String).Trim()
    $exitCode = $LASTEXITCODE
    if ($output) { Write-Host $output }
    if ($exitCode -ne 0 -and -not $AllowFailure) {
        throw "Voxveil target helper failed with exit code $exitCode.`r`n$output"
    }
    return $exitCode
}

function New-DevelopmentSigningCertificate {
    $subject = 'CN=Voxveil Development APO'
    $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $subject `
        -CertStoreLocation 'Cert:\LocalMachine\My' -KeyAlgorithm RSA -KeyLength 2048 `
        -HashAlgorithm SHA256 -NotAfter (Get-Date).AddYears(1)
    Export-Certificate -Cert $cert -FilePath $CertificatePath -Force | Out-Null
    Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\Root' | Out-Null
    Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\TrustedPublisher' | Out-Null
    $script:SigningThumbprint = $cert.Thumbprint
    return $cert
}

function Remove-DevelopmentCertificate([string]$Thumbprint, [switch]$KeepTrust) {
    if ([string]::IsNullOrWhiteSpace($Thumbprint)) { return }
    Remove-Item -LiteralPath ("Cert:\LocalMachine\My\{0}" -f $Thumbprint) -Force -ErrorAction SilentlyContinue
    if (-not $KeepTrust) {
        Remove-Item -LiteralPath ("Cert:\LocalMachine\Root\{0}" -f $Thumbprint) -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath ("Cert:\LocalMachine\TrustedPublisher\{0}" -f $Thumbprint) -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $CertificatePath -Force -ErrorAction SilentlyContinue
    }
}

function New-SignedDriverCatalog {
    param(
        [string]$PackagePath,
        [string]$CatalogName,
        [System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
    )
    $temporaryCatalog = Join-Path $env:TEMP ("voxveil-{0}-{1}.cat" -f [IO.Path]::GetFileNameWithoutExtension($CatalogName), [Guid]::NewGuid().ToString('N'))
    try {
        New-FileCatalog -Path $PackagePath -CatalogFilePath $temporaryCatalog -CatalogVersion 2 | Out-Null
        Add-VoxveilPnpCatalogAttributes -Path $temporaryCatalog -OsAttr '2:10.0'
        $signature = Set-AuthenticodeSignature -FilePath $temporaryCatalog -Certificate $Certificate -HashAlgorithm SHA256
        if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
            throw "Catalog signing failed for ${CatalogName}: $($signature.StatusMessage)"
        }
        Move-Item -LiteralPath $temporaryCatalog -Destination (Join-Path $PackagePath $CatalogName) -Force
    } finally {
        Remove-Item -LiteralPath $temporaryCatalog -Force -ErrorAction SilentlyContinue
    }
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

function Remove-VoxveilDriverPackages {
    try {
        Import-Module Dism -ErrorAction Stop
        $drivers = @(Get-WindowsDriver -Online | Where-Object { $_.ProviderName -eq 'Voxveil' })
        foreach ($driver in $drivers) {
            if ($driver.Driver) {
                Invoke-PnpUtil @('/delete-driver', [string]$driver.Driver, '/uninstall', '/force') -AllowFailure | Out-Null
            }
        }
    } catch {
        Write-Warning "Could not fully remove staged Voxveil driver packages: $($_.Exception.Message)"
    }
}

function Restart-AudioTarget([string]$InstanceId) {
    $result = Invoke-PnpUtil @('/restart-device', $InstanceId) -AllowFailure
    if ($result -ne 0) {
        Write-Warning 'Windows could not restart the target audio device automatically. Restart Windows before testing Voxveil.'
    }
    try {
        Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction Stop
        Start-Service -Name 'Audiosrv' -ErrorAction SilentlyContinue
    } catch {
        Write-Warning "Audio services could not be restarted automatically: $($_.Exception.Message). Restart Windows before testing Voxveil."
    }
}

if (-not (Test-Administrator)) { Invoke-ElevatedSelf }

try {
    if ([Environment]::OSVersion.Version.Build -lt 22000) {
        throw 'This Voxveil APO development package currently requires Windows 11 build 22000 or newer.'
    }
    foreach ($required in @(
        'VoxveilApo.dll', 'VoxveilApoCheck.exe', 'VoxveilApoTarget.exe',
        'VoxveilApo.inf', 'targets.ps1', 'extension.ps1', 'catalog.ps1', 'uninstall.ps1'
    )) {
        if (-not (Test-Path -LiteralPath (Join-Path $PackageRoot $required) -PathType Leaf)) {
            throw "Voxveil system-audio payload is missing: $required"
        }
    }

    New-Item -ItemType Directory -Path $StateRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null
    & icacls.exe $StateRoot /grant '*S-1-5-32-545:(OI)(CI)M' /T /C | Out-Null
    & icacls.exe $StateRoot /grant '*S-1-5-19:(OI)(CI)M' /T /C | Out-Null

    $targetDll = Join-Path $InstallRoot 'VoxveilApo.dll'
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApo.dll') -Destination $targetDll -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApoTarget.exe') -Destination (Join-Path $InstallRoot 'VoxveilApoTarget.exe') -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApoCheck.exe') -Destination (Join-Path $InstallRoot 'VoxveilApoCheck.exe') -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'uninstall.ps1') -Destination (Join-Path $InstallRoot 'uninstall.ps1') -Force

    Test-ApoComServer $targetDll
    Invoke-TargetHelper '--cleanup-runtime' -AllowFailure | Out-Null
    Remove-LegacyGlobalRegistration
    Remove-VoxveilDriverPackages

    $targets = @(Get-VoxveilAudioTargets -PackageRoot $PackageRoot)
    if ($targets.Count -ne 1) { throw "Expected one default render target, got $($targets.Count)." }
    $target = $targets[0]

    $workingRoot = Join-Path $env:TEMP ("voxveil-apo-package-{0}" -f [Guid]::NewGuid().ToString('N'))
    $apoPackage = Join-Path $workingRoot 'apo'
    $extensionPackage = Join-Path $workingRoot 'extension'
    New-Item -ItemType Directory -Path $apoPackage, $extensionPackage -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApo.inf') -Destination (Join-Path $apoPackage 'VoxveilApo.inf')
    Copy-Item -LiteralPath (Join-Path $PackageRoot 'VoxveilApo.dll') -Destination (Join-Path $apoPackage 'VoxveilApo.dll')
    New-VoxveilExtensionInf -Targets $targets -Path (Join-Path $extensionPackage 'VoxveilExtension.inf') | Out-Null

    $certificate = New-DevelopmentSigningCertificate
    New-SignedDriverCatalog -PackagePath $apoPackage -CatalogName 'VoxveilApo.cat' -Certificate $certificate
    New-SignedDriverCatalog -PackagePath $extensionPackage -CatalogName 'VoxveilExtension.cat' -Certificate $certificate

    $script:PackagesTouched = $true
    Invoke-PnpUtil @('/add-driver', (Join-Path $apoPackage 'VoxveilApo.inf'), '/install') | Out-Null
    Invoke-PnpUtil @('/add-driver', (Join-Path $extensionPackage 'VoxveilExtension.inf'), '/install') | Out-Null

    Enable-DevelopmentAudioGraph
    [IO.File]::WriteAllBytes($Control, [byte[]](1, 0, 100))
    Remove-Item -LiteralPath $Runtime -Force -ErrorAction SilentlyContinue

    [pscustomobject]@{
        Version = 4
        Clsid = $Clsid
        DllPath = $targetDll
        Deployment = 'componentized-apo-development'
        TargetInstanceId = [string]$target.instanceId
        TargetHardwareId = [string]$target.hardwareId
        SigningThumbprint = $script:SigningThumbprint
        InstalledAtUtc = [DateTime]::UtcNow.ToString('o')
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Marker -Encoding UTF8

    Remove-DevelopmentCertificate $script:SigningThumbprint -KeepTrust
    Restart-AudioTarget ([string]$target.instanceId)
    Remove-Item -LiteralPath $workingRoot -Recurse -Force -ErrorAction SilentlyContinue
    Write-InstallResult -Success $true -Message 'Voxveil Windows system audio component installed for the current default render device.'
    Write-Host 'Voxveil componentized APO and device extension installed. Windows Test Mode was not enabled.'
    exit 0
} catch {
    $message = $_.Exception.Message
    if ($script:PackagesTouched) { Remove-VoxveilDriverPackages }
    Remove-DevelopmentCertificate $script:SigningThumbprint
    Restore-ProtectedAudioIfCreated
    Remove-LegacyGlobalRegistration
    $details = "$message`r`nThe installer did not enable Windows TESTSIGNING or modify BCD settings."
    Write-InstallResult -Success $false -Message 'Voxveil Windows system-audio installation failed.' -Details $details
    Write-Error $details
    exit 1
}
