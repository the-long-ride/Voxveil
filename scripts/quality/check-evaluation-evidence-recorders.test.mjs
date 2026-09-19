import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const listening = readFileSync('scripts/evaluation/record-classic-dsp-listening.ps1', 'utf8');
const runtime = readFileSync('scripts/evaluation/record-windows-runtime-measurement.ps1', 'utf8');

for (const [name, script] of [
  ['Classic DSP listening recorder', listening],
  ['Windows runtime manual-measurement recorder', runtime],
]) {
  test(`${name} confines edits to JSON under the ignored measurements workspace and replaces the file atomically`, () => {
    assert.match(script, /\.local-evaluation\\classic-dsp/i);
    assert.match(script, /Join-Path\s+\$Workspace\s+'measurements'/i);
    assert.match(script, /StartsWith\(\$prefix,\s*\[StringComparison\]::OrdinalIgnoreCase\)/i);
    assert.match(script, /GetExtension\(\$evidencePath\)[\s\S]*'\.json'/i);
    assert.match(script, /\[IO\.File\]::Replace\(\$tempPath,\s*\$Path,\s*\$null\)/i);
    assert.doesNotMatch(script, /git\s+(?:add|commit|push)/i);
  });
}

test('Classic DSP listening recorder requires both profile notes and an explicit accepted/rejected decision', () => {
  assert.match(listening, /ValidateSet\('accepted',\s*'rejected'\)/i);
  assert.match(listening, /MusicPreservation/i);
  assert.match(listening, /Balanced/i);
  assert.match(listening, /ReviewMethod/i);
  assert.match(listening, /objectiveStatus\s+-ne\s+'rendered'/i);
  assert.match(listening, /subjectiveReview\.status\s*=\s*'complete'/i);
  assert.match(listening, /reviewedAtUtc/i);
  assert.match(listening, /Use -Force to replace/i);
});

test('Windows runtime recorder requires observed dropouts, latency, and a measurement method', () => {
  assert.match(runtime, /DropoutCount/i);
  assert.match(runtime, /EndToEndLatencyMs/i);
  assert.match(runtime, /LatencyMethod/i);
  assert.match(runtime, /manualMeasurements\.status\s*=\s*'complete'/i);
  assert.match(runtime, /recordedAtUtc/i);
  assert.match(runtime, /Use -Force to replace/i);
});
