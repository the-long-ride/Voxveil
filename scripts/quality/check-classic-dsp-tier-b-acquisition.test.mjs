import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const script = readFileSync('scripts/evaluation/acquire-tier-b-muffin-man.ps1', 'utf8');

test('Tier-B acquisition helper stays inside ignored evaluation workspace and verifies the exact Commons binary', () => {
  assert.match(script, /\.local-evaluation\\classic-dsp/i);
  assert.match(script, /upload\.wikimedia\.org\/wikipedia\/commons\/f\/fc\/The_Muffin_Man_/i);
  assert.match(script, /1101160L/);
  assert.match(script, /0a2ff6ab77db7995c4a8b2ed868520bd80838574/i);
  assert.match(script, /\\[Security\\.Cryptography\\.SHA1\\]::Create\\(\\)/i);
  assert.match(script, /\\[Security\\.Cryptography\\.SHA256\\]::Create\\(\\)/i);
  assert.match(script, /ComputeHash\\(\\$stream\\)/i);
  assert.doesNotMatch(script, /Get-FileHash/i);
  assert.match(script, /status\s*=\s*'approved-metadata'/i);
  assert.match(script, /tier\s*=\s*'natural-mix'/i);
  assert.match(script, /mixRecipe\s*=\s*\$null/i);
  assert.match(script, /listening\/evaluation still pending/i);
});

test('Tier-B acquisition helper writes only source and manifest files under the caller-selected workspace', () => {
  assert.match(script, /Join-Path\s+\$workspace\s+'sources'/i);
  assert.match(script, /Join-Path\s+\$workspace\s+'manifests'/i);
  assert.doesNotMatch(script, /git\s+(?:add|commit|push)/i);
});
