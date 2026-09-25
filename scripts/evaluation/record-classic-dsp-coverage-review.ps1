[CmdletBinding()]
param(
  [string]$WorkspaceRoot = '.local-evaluation\classic-dsp',

  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$MaleLeadVocalFixture,
  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$FemaleLeadVocalFixture,
  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$SparseAccompanimentFixture,
  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$DenseAccompanimentFixture,
  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$CenteredInstrumentFixture,
  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$WideStereoAmbienceFixture,
  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$MonoNearMonoFixture,

  [string]$HarmonyDoubleTrackedFixture,
  [string]$HarmonyCoverageNotApplicableReason,

  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$ReviewMethod,
  [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$Notes,

  [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-Sha256 {
  param([Parameter(Mandatory = $true)][string]$Path)
  $stream = [IO.File]::OpenRead($Path)
  try {
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
      return ([BitConverter]::ToString($hasher.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
    } finally {
      $hasher.Dispose()
    }
  } finally {
    $stream.Dispose()
  }
}

function Get-AcceptedFixtureEvidence {
  param(
    [Parameter(Mandatory = $true)][string]$FixtureId,
    [Parameter(Mandatory = $true)][string]$ManifestsDir,
    [Parameter(Mandatory = $true)][string]$MeasurementsDir
  )

  if ($FixtureId -match '[\\/]') {
    throw "Fixture ID must be an ID, not a path: $FixtureId"
  }

  $manifestPath = Join-Path $ManifestsDir ($FixtureId + '.json')
  if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Fixture manifest was not found: $manifestPath"
  }
  $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
  if ([string]$manifest.fixtureId -ne $FixtureId -or
      [string]$manifest.tier -notin @('controlled', 'natural-mix') -or
      [int]$manifest.targetSampleRate -notin @(44100, 48000)) {
    throw "Fixture manifest identity is invalid for $FixtureId."
  }
  if ([string]$manifest.status -eq 'candidate') {
    throw "Candidate fixture cannot satisfy semantic release coverage: $FixtureId"
  }

  $renderPath = Join-Path $MeasurementsDir ("{0}-{1}-render-evidence.json" -f $FixtureId, [int]$manifest.targetSampleRate)
  if (-not (Test-Path -LiteralPath $renderPath -PathType Leaf)) {
    throw "Render evidence was not found for fixture $FixtureId."
  }
  $render = Get-Content -LiteralPath $renderPath -Raw | ConvertFrom-Json
  if ([string]$render.fixtureId -ne $FixtureId -or
      [int]$render.sampleRate -ne [int]$manifest.targetSampleRate -or
      [string]$render.subjectiveReview.status -ne 'complete' -or
      [string]$render.subjectiveReview.decision -ne 'accepted') {
    throw "Fixture does not have completed accepted listening evidence: $FixtureId"
  }

  return [ordered]@{
    status = 'covered'
    fixtureId = $FixtureId
    tier = [string]$manifest.tier
    sampleRate = [int]$manifest.targetSampleRate
    manifestSha256 = Get-Sha256 $manifestPath
    renderEvidenceSha256 = Get-Sha256 $renderPath
  }
}

if ($HarmonyDoubleTrackedFixture -and $HarmonyCoverageNotApplicableReason) {
  throw 'Specify either -HarmonyDoubleTrackedFixture or -HarmonyCoverageNotApplicableReason, not both.'
}
if (-not $HarmonyDoubleTrackedFixture -and -not $HarmonyCoverageNotApplicableReason) {
  throw 'Harmony/double-tracked coverage requires either an accepted fixture or an explicit licensing-based not-applicable reason.'
}

$workspace = [IO.Path]::GetFullPath($WorkspaceRoot)
$manifestsDir = Join-Path $workspace 'manifests'
$measurementsDir = Join-Path $workspace 'measurements'
foreach ($directory in @($manifestsDir, $measurementsDir)) {
  if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
    throw "Classic DSP evaluation directory was not found: $directory"
  }
}

$categories = [ordered]@{
  maleLeadVocal = Get-AcceptedFixtureEvidence -FixtureId $MaleLeadVocalFixture -ManifestsDir $manifestsDir -MeasurementsDir $measurementsDir
  femaleLeadVocal = Get-AcceptedFixtureEvidence -FixtureId $FemaleLeadVocalFixture -ManifestsDir $manifestsDir -MeasurementsDir $measurementsDir
  sparseAccompaniment = Get-AcceptedFixtureEvidence -FixtureId $SparseAccompanimentFixture -ManifestsDir $manifestsDir -MeasurementsDir $measurementsDir
  denseAccompaniment = Get-AcceptedFixtureEvidence -FixtureId $DenseAccompanimentFixture -ManifestsDir $manifestsDir -MeasurementsDir $measurementsDir
  centeredLowFrequencyOrInstrument = Get-AcceptedFixtureEvidence -FixtureId $CenteredInstrumentFixture -ManifestsDir $manifestsDir -MeasurementsDir $measurementsDir
  wideStereoAmbience = Get-AcceptedFixtureEvidence -FixtureId $WideStereoAmbienceFixture -ManifestsDir $manifestsDir -MeasurementsDir $measurementsDir
  monoNearMono = Get-AcceptedFixtureEvidence -FixtureId $MonoNearMonoFixture -ManifestsDir $manifestsDir -MeasurementsDir $measurementsDir
}

if ($HarmonyDoubleTrackedFixture) {
  $categories.harmonyDoubleTracked = Get-AcceptedFixtureEvidence -FixtureId $HarmonyDoubleTrackedFixture -ManifestsDir $manifestsDir -MeasurementsDir $measurementsDir
} else {
  $categories.harmonyDoubleTracked = [ordered]@{
    status = 'not-applicable'
    reason = $HarmonyCoverageNotApplicableReason.Trim()
  }
}

$timestamp = [DateTime]::UtcNow
$output = Join-Path $measurementsDir ("classic-dsp-coverage-review-{0}.json" -f $timestamp.ToString('yyyyMMddTHHmmssZ'))
if ((Test-Path -LiteralPath $output) -and -not $Force) {
  throw "Coverage review already exists at $output; use -Force to replace it."
}

$review = [ordered]@{
  schemaVersion = 1
  reviewedAtUtc = $timestamp.ToString('o')
  reviewMethod = $ReviewMethod.Trim()
  notes = $Notes.Trim()
  categories = $categories
  statement = 'Coverage categories are explicit human review labels over already accepted fixtures; the recorder does not infer semantic content from audio, file names, or notes.'
}

$json = $review | ConvertTo-Json -Depth 10
[IO.File]::WriteAllText($output, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))

Write-Host 'Classic DSP semantic coverage review recorded.'
Write-Host "Evidence: $output"
