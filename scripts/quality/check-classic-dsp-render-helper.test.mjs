import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const script = readFileSync('scripts/evaluation/render-classic-dsp-fixture.ps1', 'utf8');

test('Classic DSP local renderer runs all profiles at Vocal 0 and keeps outputs in the ignored workspace', () => {
  assert.match(script, /\.local-evaluation\\classic-dsp/i);
  assert.match(script, /ValidateSet\('controlled',\s*'natural-mix'\)/i);
  assert.match(script, /Resolve-Application\s+'ffprobe\.exe'/i);
  assert.match(script, /show_entries',\s*'stream=sample_rate'/i);
  assert.match(script, /sourceSampleRate\s+-ne\s+\$SampleRate/i);
  assert.match(script, /native-rate input rather than resampled material/i);
  assert.match(script, /--profile',\s*'music-preservation'/i);
  assert.match(script, /--profile',\s*'balanced'/i);
  assert.match(script, /--profile',\s*'strong'/i);
  assert.equal((script.match(/'--vocal',\s*'0'/g) ?? []).length, 3);
  assert.match(script, /Join-Path\s+\$workspace\s+'fixtures'/i);
  assert.match(script, /Join-Path\s+\$workspace\s+'renders'/i);
  assert.match(script, /Join-Path\s+\$workspace\s+'measurements'/i);
});

test('Classic DSP local renderer verifies aligned finite raw output and records objective evidence without claiming acceptance', () => {
  assert.match(script, /Latency-compensated render size mismatch/i);
  assert.match(script, /IsNaN/i);
  assert.match(script, /IsInfinity/i);
  assert.match(script, /\[System\.Security\.Cryptography\.SHA256\]::Create\(\)/i);
  assert.match(script, /ComputeHash\(\$stream\)/i);
  assert.doesNotMatch(script, /Get-FileHash/i);
  assert.match(script, /leftRightCorrelation/i);
  assert.match(script, /midRms/i);
  assert.match(script, /sideRms/i);
  assert.match(script, /tier\s*=\s*\$Tier/i);
  assert.match(script, /nativeSampleRate\s*=\s*\$sourceSampleRate/i);
  assert.match(script, /frameCountParity\s*=\s*\$true/i);
  assert.match(script, /objectiveStatus\s*=\s*'rendered'/i);
  assert.match(script, /status\s*=\s*'pending'/i);
  assert.match(script, /does not establish subjective acceptance/i);
});
