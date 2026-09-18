import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const script = readFileSync('scripts/evaluation/prepare-tier-a-controlled-fixture.ps1', 'utf8');

test('Tier-A builder confines sources and outputs to the ignored evaluation workspace', () => {
  assert.match(script, /\.local-evaluation\\classic-dsp/i);
  assert.match(script, /Join-Path\s+\$workspace\s+'sources'/i);
  assert.match(script, /Join-Path\s+\$workspace\s+'fixtures'/i);
  assert.match(script, /Join-Path\s+\$workspace\s+'manifests'/i);
  assert.match(script, /StartsWith\(\$prefix,\s*\[StringComparison\]::OrdinalIgnoreCase\)/i);
  assert.doesNotMatch(script, /git\s+(?:add|commit|push)/i);
});

test('Tier-A builder freezes the VocalSet and URMP native-rate boundary', () => {
  assert.match(script, /dataset\s*=\s*'VocalSet'/i);
  assert.match(script, /dataset\s*=\s*'URMP'/i);
  assert.match(script, /license\s*=\s*'CC-BY-4\.0'/i);
  assert.match(script, /license\s*=\s*'CC0-1\.0'/i);
  assert.match(script, /TargetSampleRate\s+-eq\s+44100[\s\S]*vocalInfo\.sampleRate\s+-ne\s+44100/i);
  assert.match(script, /TargetSampleRate\s+-eq\s+48000[\s\S]*accompanimentInfo\.sampleRate\s+-ne\s+48000/i);
  assert.match(script, /VocalSet source must be mono/i);
});

test('Tier-A builder records a deterministic non-normalizing mix recipe and hashes', () => {
  assert.match(script, /amix=inputs=2:duration=shortest:normalize=0/i);
  assert.doesNotMatch(script, /loudnorm/i);
  assert.match(script, /\[System\.Security\.Cryptography\.SHA256\]::Create\(\)/i);
  assert.match(script, /tier\s*=\s*'controlled'/i);
  assert.match(script, /status\s*=\s*'prepared'/i);
  assert.match(script, /mixRecipe\s*=\s*\[ordered\]@\{/i);
  assert.match(script, /normalization\s*=\s*'none'/i);
  assert.match(script, /fixtureSha256\s*=\s*\$fixtureSha256/i);
  assert.match(script, /Use -Force to replace/i);
});
