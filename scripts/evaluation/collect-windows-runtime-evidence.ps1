[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\classic-dsp',

  [ValidateSet(44100, 48000)]
  [int]$SampleRate = 48000,

  [ValidateSet('music-preservation', 'balanced')]
  [string]$Profile = 'music-preservation',

  [ValidateRange(0, 100)]
  [int]$Vocal = 0,

  [string]$SourceEndpoint = '',
  [string]$PhysicalOutput = '',
  [string]$ProcessName = 'Voxveil',

  [ValidateRange(0, 60)]
  [int]$CpuSampleSeconds = 0
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$measurements = Join-Path $workspace 'measurements'
New-Item -ItemType Directory -Force -Path $measurements | Out-Null

$os = Get-CimInstance Win32_OperatingSystem
$processors = @(Get-CimInstance Win32_Processor)
if ($processors.Count -eq 0) {
  throw 'No Win32_Processor records were returned.'
}

$logicalProcessors = [int](($processors | Measure-Object -Property NumberOfLogicalProcessors -Sum).Sum)
if ($logicalProcessors -lt 1) {
  throw 'Logical processor count could not be determined.'
}

$cpuNames = @($processors | ForEach-Object { $_.Name.Trim() } | Sort-Object -Unique)

$secureBoot = [ordered]@{
  status = 'unavailable'
  enabled = $null
  errorType = $null
}
$secureBootCommand = Get-Command Confirm-SecureBootUEFI -ErrorAction SilentlyContinue
if ($secureBootCommand) {
  try {
    $secureBoot.enabled = [bool](Confirm-SecureBootUEFI)
    $secureBoot.status = 'read'
  }
  catch {
    $secureBoot.status = 'error'
    $secureBoot.errorType = $_.Exception.GetType().FullName
  }
}

$systemDirectory = [Environment]::SystemDirectory
if (-not $systemDirectory) {
  throw 'Windows system directory could not be resolved.'
}
$bcdedit = Join-Path $systemDirectory 'bcdedit.exe'
if (-not (Test-Path $bcdedit -PathType Leaf)) {
  throw "bcdedit.exe was not found at $bcdedit."
}

$bcdOutput = @(& $bcdedit /enum '{current}' 2>&1)
$bcdExitCode = $LASTEXITCODE
$testSigningLine = @($bcdOutput | Where-Object { "$_" -match '^\s*testsigning\s+' } | Select-Object -First 1)
$testSigningRaw = $null
$testSigningEnabled = $null
if ($testSigningLine.Count -gt 0) {
  $testSigningRaw = ("$($testSigningLine[0])" -replace '^\s*testsigning\s+', '').Trim()
  if ($testSigningRaw -match '^(yes|on|true|1)$') {
    $testSigningEnabled = $true
  }
  elseif ($testSigningRaw -match '^(no|off|false|0)$') {
    $testSigningEnabled = $false
  }
}

$cpuSample = [ordered]@{
  status = 'not-requested'
  processName = $ProcessName
  sampleSeconds = $CpuSampleSeconds
  normalizedPercent = $null
}
if ($CpuSampleSeconds -gt 0) {
  $startProcesses = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue)
  if ($startProcesses.Count -eq 0) {
    $cpuSample.status = 'process-not-found'
  }
  else {
    $startCpu = [double](($startProcesses | ForEach-Object { $_.TotalProcessorTime.TotalSeconds } | Measure-Object -Sum).Sum)
    $stopwatch = [Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds $CpuSampleSeconds
    $endProcesses = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue)
    $stopwatch.Stop()

    if ($endProcesses.Count -eq 0) {
      $cpuSample.status = 'process-ended'
    }
    else {
      $endCpu = [double](($endProcesses | ForEach-Object { $_.TotalProcessorTime.TotalSeconds } | Measure-Object -Sum).Sum)
      $elapsed = $stopwatch.Elapsed.TotalSeconds
      if ($elapsed -le 0 -or $endCpu -lt $startCpu) {
        $cpuSample.status = 'invalid-sample'
      }
      else {
        $normalized = (($endCpu - $startCpu) / $elapsed / $logicalProcessors) * 100.0
        $cpuSample.normalizedPercent = [Math]::Round($normalized, 3)
        $cpuSample.status = 'sampled'
      }
    }
  }
}

$timestamp = [DateTime]::UtcNow
$fileStamp = $timestamp.ToString('yyyyMMddTHHmmssZ')
$evidencePath = Join-Path $measurements "windows-runtime-$SampleRate-$Profile-$fileStamp.json"

$evidence = [ordered]@{
  generatedAtUtc = $timestamp.ToString('o')
  machine = [ordered]@{
    windowsCaption = $os.Caption
    windowsVersion = $os.Version
    windowsBuild = $os.BuildNumber
    osArchitecture = $os.OSArchitecture
    cpuModels = $cpuNames
    logicalProcessorCount = $logicalProcessors
    secureBoot = $secureBoot
    testSigning = [ordered]@{
      bcdeditExitCode = $bcdExitCode
      rawValue = $testSigningRaw
      enabled = $testSigningEnabled
    }
  }
  route = [ordered]@{
    sourceEndpoint = $SourceEndpoint
    physicalOutput = $PhysicalOutput
    sampleRate = $SampleRate
    profile = $Profile
    vocal = $Vocal
  }
  cpu = $cpuSample
  manualMeasurements = [ordered]@{
    status = 'pending'
    dropoutCount = $null
    endToEndLatencyMs = $null
    latencyMethod = ''
    notes = ''
  }
  notes = 'Machine/runtime capture only. Dropout and end-to-end latency remain manual measurements until explicitly recorded.'
}

$json = $evidence | ConvertTo-Json -Depth 10
[IO.File]::WriteAllText($evidencePath, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))

Write-Host "Windows build: $($os.BuildNumber)"
Write-Host "Logical processors: $logicalProcessors"
Write-Host "Secure Boot status: $($secureBoot.status)"
Write-Host "TESTSIGNING parsed value: $testSigningEnabled"
Write-Host "CPU sample status: $($cpuSample.status)"
Write-Host "Evidence: $evidencePath"
