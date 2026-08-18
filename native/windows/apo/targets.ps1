Set-StrictMode -Version 2.0

function Get-VoxveilAudioTargets {
    param(
        [string]$PackageRoot = $PSScriptRoot
    )

    $helper = Join-Path $PackageRoot 'VoxveilApoTarget.exe'
    if (-not (Test-Path -LiteralPath $helper -PathType Leaf)) {
        throw "Voxveil audio target helper is missing: $helper"
    }

    $stdoutFile = Join-Path $env:TEMP ("voxveil-targets-{0}.json" -f [Guid]::NewGuid().ToString('N'))
    $stderrFile = Join-Path $env:TEMP ("voxveil-targets-{0}.err" -f [Guid]::NewGuid().ToString('N'))
    try {
        $process = Start-Process -FilePath $helper -NoNewWindow -Wait -PassThru `
            -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile
        $stdout = if (Test-Path -LiteralPath $stdoutFile) { Get-Content -LiteralPath $stdoutFile -Raw } else { '' }
        $stderr = if (Test-Path -LiteralPath $stderrFile) { Get-Content -LiteralPath $stderrFile -Raw } else { '' }
        if ($process.ExitCode -ne 0) {
            throw ("Voxveil audio target discovery failed with exit code {0}.`r`n{1}" -f $process.ExitCode, $stderr.Trim())
        }
        if ([string]::IsNullOrWhiteSpace($stdout)) {
            throw 'Voxveil audio target discovery returned no data.'
        }

        $targets = @($stdout | ConvertFrom-Json)
        $validated = @()
        foreach ($target in $targets) {
            $instanceId = [string]$target.instanceId
            $hardwareId = [string]$target.hardwareId
            $topologyRefs = @($target.topologyRefs | ForEach-Object { [string]$_ })
            if ([string]::IsNullOrWhiteSpace($instanceId)) {
                throw 'Voxveil audio target discovery returned a target without an instance ID.'
            }
            if ([string]::IsNullOrWhiteSpace($hardwareId)) {
                throw "Voxveil audio target $instanceId has no hardware ID."
            }
            if ($topologyRefs.Count -eq 0) {
                throw "Voxveil audio target $instanceId has no topology interface."
            }
            $validated += [pscustomobject]@{
                instanceId = $instanceId
                hardwareId = $hardwareId
                topologyRefs = $topologyRefs
            }
        }
        if ($validated.Count -eq 0) {
            throw 'No usable Windows render audio targets were discovered.'
        }
        return $validated
    } finally {
        Remove-Item -LiteralPath $stdoutFile -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $stderrFile -Force -ErrorAction SilentlyContinue
    }
}
