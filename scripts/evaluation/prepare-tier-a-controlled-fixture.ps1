[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\classic-dsp',

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$VocalSource,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$AccompanimentSource,

  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9._-]*$')]
  [string]$FixtureId,

  [Parameter(Mandatory = $true)]
  [ValidateSet(44100, 48000)]
  [int]$TargetSampleRate,

  [ValidateRange(0, 86400)]
  [double]$VocalStartSeconds = 0,

  [ValidateRange(0, 86400)]
  [double]$AccompanimentStartSeconds = 0,

  [Parameter(Mandatory = $true)]
  [ValidateRange(0.1, 300)]
  [double]$DurationSeconds,

  [ValidateRange(-60, 24)]
  [double]$VocalGainDb = -6,

  [ValidateRange(-60, 24)]
  [double]$AccompanimentGainDb = 0,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$VocalSourceRecord,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$AccompanimentSourceRecord,

  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[0-9]{4}-[0-9]{2}-[0-9]{2}$')]
  [string]$LicenseCheckedOn,

  [string]$Notes = '',
  [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Resolve-Application {
  param([Parameter(Mandatory = $true)][string]$Name)
  $command = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $command) {
    throw "$Name is required for controlled Classic DSP fixture preparation but was not found."
  }
  return $command.Source
}

function Resolve-SourcePath {
  param(
    [Parameter(Mandatory = $true)][string]$SourceRoot,
    [Parameter(Mandatory = $true)][string]$RequestedPath
  )

  $sourceRootFull = [IO.Path]::GetFullPath($SourceRoot)
  $sourcePath = [IO.Path]::GetFullPath($RequestedPath)
  $prefix = $sourceRootFull.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar

  if (-not $sourcePath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Controlled-fixture source must remain under $sourceRootFull."
  }
  if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
    throw "Controlled-fixture source was not found: $sourcePath"
  }

  return $sourcePath
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

  $stream = [IO.File]::OpenRead($Path)
  try {
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try {
      return ([BitConverter]::ToString($hasher.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
    }
    finally {
      $hasher.Dispose()
    }
  }
  finally {
    $stream.Dispose()
  }
}

function Get-AudioStreamInfo {
  param(
    [Parameter(Mandatory = $true)][string]$Ffprobe,
    [Parameter(Mandatory = $true)][string]$Path
  )

  $output = @(& $Ffprobe @(
    '-v', 'error',
    '-select_streams', 'a:0',
    '-show_entries', 'stream=sample_rate,channels',
    '-of', 'json',
    $Path
  ))
  if ($LASTEXITCODE -ne 0) {
    throw "ffprobe.exe failed while reading $Path with exit code $LASTEXITCODE."
  }

  $parsed = ($output -join [Environment]::NewLine) | ConvertFrom-Json
  if (-not $parsed.streams -or $parsed.streams.Count -ne 1) {
    throw "Could not resolve exactly one primary audio stream from $Path."
  }

  [int]$sampleRate = 0
  [int]$channels = 0
  if (-not [int]::TryParse("$($parsed.streams[0].sample_rate)", [ref]$sampleRate) -or $sampleRate -le 0) {
    throw "Could not determine source sample rate for $Path."
  }
  if (-not [int]::TryParse("$($parsed.streams[0].channels)", [ref]$channels) -or $channels -le 0) {
    throw "Could not determine channel count for $Path."
  }

  return [ordered]@{
    sampleRate = $sampleRate
    channels = $channels
  }
}

function Get-FixtureDuration {
  param(
    [Parameter(Mandatory = $true)][string]$Ffprobe,
    [Parameter(Mandatory = $true)][string]$Path
  )

  $output = @(& $Ffprobe @(
    '-v', 'error',
    '-show_entries', 'format=duration',
    '-of', 'default=noprint_wrappers=1:nokey=1',
    $Path
  ))
  if ($LASTEXITCODE -ne 0) {
    throw "ffprobe.exe failed while reading fixture duration with exit code $LASTEXITCODE."
  }

  $text = (($output | ForEach-Object { "$_".Trim() } | Where-Object { $_ }) -join '')
  [double]$duration = 0
  if (-not [double]::TryParse(
      $text,
      [Globalization.NumberStyles]::Float,
      [Globalization.CultureInfo]::InvariantCulture,
      [ref]$duration
    ) -or $duration -le 0) {
    throw "Could not determine fixture duration from ffprobe output: $text"
  }

  return $duration
}

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$sources = Join-Path $workspace 'sources'
$fixtures = Join-Path $workspace 'fixtures'
$manifests = Join-Path $workspace 'manifests'
New-Item -ItemType Directory -Force -Path $sources, $fixtures, $manifests | Out-Null

$vocalPath = Resolve-SourcePath -SourceRoot $sources -RequestedPath $VocalSource
$accompanimentPath = Resolve-SourcePath -SourceRoot $sources -RequestedPath $AccompanimentSource

$fixturePath = Join-Path $fixtures "$FixtureId.wav"
$manifestPath = Join-Path $manifests "$FixtureId.json"
foreach ($path in @($fixturePath, $manifestPath)) {
  if ((Test-Path -LiteralPath $path -PathType Leaf) -and -not $Force) {
    throw "Controlled fixture output already exists: $path. Use -Force to replace it."
  }
}

$ffmpeg = Resolve-Application 'ffmpeg.exe'
$ffprobe = Resolve-Application 'ffprobe.exe'

$vocalInfo = Get-AudioStreamInfo -Ffprobe $ffprobe -Path $vocalPath
$accompanimentInfo = Get-AudioStreamInfo -Ffprobe $ffprobe -Path $accompanimentPath

if ($vocalInfo.channels -ne 1) {
  throw "VocalSet source must be mono for the frozen controlled-fixture recipe; found $($vocalInfo.channels) channels."
}
if ($TargetSampleRate -eq 44100 -and $vocalInfo.sampleRate -ne 44100) {
  throw "44.1 kHz controlled fixtures must preserve a native 44.1 kHz VocalSet source; found $($vocalInfo.sampleRate) Hz."
}
if ($TargetSampleRate -eq 48000 -and $accompanimentInfo.sampleRate -ne 48000) {
  throw "48 kHz controlled fixtures must preserve a native 48 kHz URMP accompaniment; found $($accompanimentInfo.sampleRate) Hz."
}

$invariant = [Globalization.CultureInfo]::InvariantCulture
$vocalStart = $VocalStartSeconds.ToString('0.###', $invariant)
$accompanimentStart = $AccompanimentStartSeconds.ToString('0.###', $invariant)
$duration = $DurationSeconds.ToString('0.###', $invariant)
$vocalGain = $VocalGainDb.ToString('0.###', $invariant)
$accompanimentGain = $AccompanimentGainDb.ToString('0.###', $invariant)

$filter = ('[0:a]aresample={0},pan=stereo|c0=c0|c1=c0,volume={1}dB[v];' +
  '[1:a]aresample={0},aformat=channel_layouts=stereo,volume={2}dB[m];' +
  '[m][v]amix=inputs=2:duration=shortest:normalize=0[out]') -f $TargetSampleRate, $vocalGain, $accompanimentGain

try {
  Invoke-Checked $ffmpeg @(
    '-v', 'error',
    '-ss', $vocalStart,
    '-t', $duration,
    '-i', $vocalPath,
    '-ss', $accompanimentStart,
    '-t', $duration,
    '-i', $accompanimentPath,
    '-filter_complex', $filter,
    '-map', '[out]',
    '-ar', "$TargetSampleRate",
    '-ac', '2',
    '-c:a', 'pcm_f32le',
    '-map_metadata', '-1',
    '-y', $fixturePath
  )

  $fixtureInfo = Get-AudioStreamInfo -Ffprobe $ffprobe -Path $fixturePath
  if ($fixtureInfo.sampleRate -ne $TargetSampleRate -or $fixtureInfo.channels -ne 2) {
    throw "Prepared fixture format mismatch: expected stereo $TargetSampleRate Hz, got $($fixtureInfo.channels) channels at $($fixtureInfo.sampleRate) Hz."
  }

  $actualDuration = Get-FixtureDuration -Ffprobe $ffprobe -Path $fixturePath
  $ffmpegVersion = @(& $ffmpeg -version | Select-Object -First 1)
  if ($LASTEXITCODE -ne 0 -or -not $ffmpegVersion) {
    throw 'Could not record the FFmpeg version.'
  }

  $vocalSha256 = Get-Sha256 $vocalPath
  $accompanimentSha256 = Get-Sha256 $accompanimentPath
  $fixtureSha256 = Get-Sha256 $fixturePath

  $preparationCommand = "prepare-tier-a-controlled-fixture.ps1 -FixtureId $FixtureId -TargetSampleRate $TargetSampleRate " +
    "-VocalStartSeconds $vocalStart -AccompanimentStartSeconds $accompanimentStart -DurationSeconds $duration " +
    "-VocalGainDb $vocalGain -AccompanimentGainDb $accompanimentGain"

  $manifest = [ordered]@{
    fixtureId = $FixtureId
    tier = 'controlled'
    status = 'prepared'
    targetSampleRate = $TargetSampleRate
    durationSeconds = [Math]::Round($actualDuration, 6)
    sources = @(
      [ordered]@{
        role = 'vocal'
        dataset = 'VocalSet'
        sourceRecord = $VocalSourceRecord
        sourceFile = "sources/$([IO.Path]::GetFileName($vocalPath))"
        license = 'CC-BY-4.0'
        licenseCheckedOn = $LicenseCheckedOn
        sourceSha256 = $vocalSha256
      },
      [ordered]@{
        role = 'accompaniment'
        dataset = 'URMP'
        sourceRecord = $AccompanimentSourceRecord
        sourceFile = "sources/$([IO.Path]::GetFileName($accompanimentPath))"
        license = 'CC0-1.0'
        licenseCheckedOn = $LicenseCheckedOn
        sourceSha256 = $accompanimentSha256
      }
    )
    mixRecipe = [ordered]@{
      vocalStartSeconds = $VocalStartSeconds
      accompanimentStartSeconds = $AccompanimentStartSeconds
      vocalGainDb = $VocalGainDb
      accompanimentGainDb = $AccompanimentGainDb
      vocalPan = 'center'
      normalization = 'none'
      preparationCommand = $preparationCommand
      preparationToolVersion = "$ffmpegVersion"
    }
    fixtureSha256 = $fixtureSha256
    notes = if ($Notes) { $Notes } else { 'Controlled fixture prepared; profile rendering, measurements, and listening acceptance remain pending.' }
  }

  $json = $manifest | ConvertTo-Json -Depth 10
  [IO.File]::WriteAllText($manifestPath, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
}
catch {
  if (Test-Path -LiteralPath $fixturePath -PathType Leaf) {
    Remove-Item -LiteralPath $fixturePath -Force -ErrorAction SilentlyContinue
  }
  if (Test-Path -LiteralPath $manifestPath -PathType Leaf) {
    Remove-Item -LiteralPath $manifestPath -Force -ErrorAction SilentlyContinue
  }
  throw
}

Write-Host "Controlled fixture: $fixturePath"
Write-Host "Fixture SHA-256: $fixtureSha256"
Write-Host "Manifest: $manifestPath"
