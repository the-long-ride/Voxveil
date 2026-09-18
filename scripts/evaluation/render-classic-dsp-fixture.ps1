[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$Input,

  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9._-]*$')]
  [string]$FixtureId,

  [ValidateSet(44100, 48000)]
  [int]$SampleRate = 48000,

  [string]$WorkspaceRoot = '.local-evaluation\classic-dsp'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Resolve-Application {
  param([Parameter(Mandatory = $true)][string]$Name)
  $command = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $command) {
    throw "$Name is required for Classic DSP evaluation but was not found."
  }
  return $command.Source
}

function Invoke-Checked {
  param(
    [Parameter(Mandatory = $true)][string]$FilePath,
    [Parameter(Mandatory = $true)][string[]]$Arguments
  )
  & $FilePath @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$FilePath failed with exit code $LASTEXITCODE."
  }
}

function Get-Sha256 {
  param([Parameter(Mandatory = $true)][string]$Path)
  return (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-RawMetrics {
  param([Parameter(Mandatory = $true)][string]$Path)

  $bytes = [IO.File]::ReadAllBytes($Path)
  if (($bytes.Length % 8) -ne 0) {
    throw "$Path is not complete stereo f32le data."
  }

  $frames = [long]($bytes.Length / 8)
  if ($frames -eq 0) {
    throw "$Path contains no audio frames."
  }

  [double]$sumLeft2 = 0
  [double]$sumRight2 = 0
  [double]$sumCross = 0
  [double]$sumMid2 = 0
  [double]$sumSide2 = 0
  [double]$peak = 0

  for ($offset = 0; $offset -lt $bytes.Length; $offset += 8) {
    [single]$left = [BitConverter]::ToSingle($bytes, $offset)
    [single]$right = [BitConverter]::ToSingle($bytes, $offset + 4)
    if ([single]::IsNaN($left) -or [single]::IsInfinity($left) -or
        [single]::IsNaN($right) -or [single]::IsInfinity($right)) {
      throw "$Path contains a non-finite sample at byte offset $offset."
    }

    $leftD = [double]$left
    $rightD = [double]$right
    $mid = ($leftD + $rightD) * 0.5
    $side = ($leftD - $rightD) * 0.5

    $sumLeft2 += $leftD * $leftD
    $sumRight2 += $rightD * $rightD
    $sumCross += $leftD * $rightD
    $sumMid2 += $mid * $mid
    $sumSide2 += $side * $side
    $peak = [Math]::Max($peak, [Math]::Max([Math]::Abs($leftD), [Math]::Abs($rightD)))
  }

  $denominator = [Math]::Sqrt($sumLeft2 * $sumRight2)
  $correlation = if ($denominator -gt 0) { $sumCross / $denominator } else { 0.0 }

  return [ordered]@{
    frames = $frames
    bytes = [long]$bytes.Length
    peakAbsolute = [Math]::Round($peak, 9)
    rms = [Math]::Round([Math]::Sqrt(($sumLeft2 + $sumRight2) / (2.0 * $frames)), 9)
    midRms = [Math]::Round([Math]::Sqrt($sumMid2 / $frames), 9)
    sideRms = [Math]::Round([Math]::Sqrt($sumSide2 / $frames), 9)
    leftRightCorrelation = [Math]::Round($correlation, 9)
  }
}

$inputPath = [IO.Path]::GetFullPath($Input)
if (-not (Test-Path $inputPath -PathType Leaf)) {
  throw "Evaluation input was not found: $inputPath"
}

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$fixtures = Join-Path $workspace 'fixtures'
$renders = Join-Path $workspace 'renders'
$measurements = Join-Path $workspace 'measurements'
New-Item -ItemType Directory -Force -Path $fixtures, $renders, $measurements | Out-Null

$ffmpeg = Resolve-Application 'ffmpeg.exe'
$cargo = Resolve-Application 'cargo.exe'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))

$baseName = "$FixtureId-$SampleRate"
$fixtureRaw = Join-Path $fixtures "$baseName.f32"
$fixtureWav = Join-Path $fixtures "$baseName.wav"
$musicRaw = Join-Path $renders "$baseName-music-preservation.f32"
$musicWav = Join-Path $renders "$baseName-music-preservation.wav"
$balancedRaw = Join-Path $renders "$baseName-balanced.f32"
$balancedWav = Join-Path $renders "$baseName-balanced.wav"
$evidencePath = Join-Path $measurements "$baseName-render-evidence.json"

Invoke-Checked $ffmpeg @(
  '-v', 'error',
  '-i', $inputPath,
  '-map_metadata', '-1',
  '-ac', '2',
  '-ar', "$SampleRate",
  '-f', 'f32le',
  '-y', $fixtureRaw
)

Invoke-Checked $ffmpeg @(
  '-v', 'error',
  '-f', 'f32le',
  '-ac', '2',
  '-ar', "$SampleRate",
  '-i', $fixtureRaw,
  '-c:a', 'pcm_f32le',
  '-y', $fixtureWav
)

Push-Location $repoRoot
try {
  Invoke-Checked $cargo @(
    'run', '--quiet', '-p', 'voxveil-dsp', '--example', 'classic_dsp_raw', '--',
    '--input', $fixtureRaw,
    '--output', $musicRaw,
    '--sample-rate', "$SampleRate",
    '--vocal', '0',
    '--profile', 'music-preservation'
  )

  Invoke-Checked $cargo @(
    'run', '--quiet', '-p', 'voxveil-dsp', '--example', 'classic_dsp_raw', '--',
    '--input', $fixtureRaw,
    '--output', $balancedRaw,
    '--sample-rate', "$SampleRate",
    '--vocal', '0',
    '--profile', 'balanced'
  )
}
finally {
  Pop-Location
}

$fixtureBytes = (Get-Item $fixtureRaw).Length
$musicBytes = (Get-Item $musicRaw).Length
$balancedBytes = (Get-Item $balancedRaw).Length
if ($musicBytes -ne $fixtureBytes -or $balancedBytes -ne $fixtureBytes) {
  throw "Latency-compensated render size mismatch: input=$fixtureBytes music=$musicBytes balanced=$balancedBytes."
}

foreach ($render in @(
  @{ Raw = $musicRaw; Wav = $musicWav },
  @{ Raw = $balancedRaw; Wav = $balancedWav }
)) {
  Invoke-Checked $ffmpeg @(
    '-v', 'error',
    '-f', 'f32le',
    '-ac', '2',
    '-ar', "$SampleRate",
    '-i', $render.Raw,
    '-c:a', 'pcm_f32le',
    '-y', $render.Wav
  )
}

$sourceSha256 = Get-Sha256 $inputPath
$fixtureSha256 = Get-Sha256 $fixtureRaw
$musicSha256 = Get-Sha256 $musicRaw
$balancedSha256 = Get-Sha256 $balancedRaw

$evidence = [ordered]@{
  fixtureId = $FixtureId
  sampleRate = $SampleRate
  vocal = 0
  generatedAtUtc = [DateTime]::UtcNow.ToString('o')
  source = [ordered]@{
    fileName = [IO.Path]::GetFileName($inputPath)
    sha256 = $sourceSha256
  }
  input = [ordered]@{
    rawFile = "fixtures/$baseName.f32"
    wavFile = "fixtures/$baseName.wav"
    sha256 = $fixtureSha256
    metrics = Get-RawMetrics $fixtureRaw
  }
  renders = [ordered]@{
    musicPreservation = [ordered]@{
      rawFile = "renders/$baseName-music-preservation.f32"
      wavFile = "renders/$baseName-music-preservation.wav"
      sha256 = $musicSha256
      metrics = Get-RawMetrics $musicRaw
    }
    balanced = [ordered]@{
      rawFile = "renders/$baseName-balanced.f32"
      wavFile = "renders/$baseName-balanced.wav"
      sha256 = $balancedSha256
      metrics = Get-RawMetrics $balancedRaw
    }
  }
  frameCountParity = $true
  objectiveStatus = 'rendered'
  subjectiveReview = [ordered]@{
    status = 'pending'
    musicPreservation = ''
    balanced = ''
    notes = ''
  }
  notes = 'Offline render evidence only. This file does not establish subjective acceptance, Windows runtime performance, or release qualification.'
}

$json = $evidence | ConvertTo-Json -Depth 10
[IO.File]::WriteAllText($evidencePath, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))

Write-Host "Input SHA-256: $fixtureSha256"
Write-Host "Music preservation SHA-256: $musicSha256"
Write-Host "Balanced SHA-256: $balancedSha256"
Write-Host "Evidence: $evidencePath"
