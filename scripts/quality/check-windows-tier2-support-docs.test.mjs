import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('current Tier 2 docs keep the first-party driver on Windows 11 build 22621+', () => {
  for (const path of [
    'README.md',
    'docs/specs/platform/windows.md',
    'docs/testing/windows-signed-virtual-driver.md',
    'docs/release/windows-driver-signing.md',
  ]) {
    const text = read(path);
    assert.match(text, /22621/i, `${path} must state the Tier 2 build floor`);
    assert.match(text, /Windows 10[\s\S]{0,180}Tier 1|Tier 1[\s\S]{0,180}Windows 10/i, `${path} must route Windows 10 to Tier 1`);
  }
});

test('authoritative Tier 2 corrections supersede undecorated INF guidance', () => {
  const corrections = read('docs/superpowers/plans/2026-09-14-windows-signed-audio-paths-review-notes.md');

  assert.match(corrections, /Tier 2[\s\S]{0,1200}22621/i);
  assert.match(corrections, /undecorated[\s\S]{0,300}superseded|superseded[\s\S]{0,300}undecorated/i);
  assert.match(corrections, /Windows 10[\s\S]{0,300}Tier 1|Tier 1[\s\S]{0,300}Windows 10/i);
  assert.match(corrections, /Inf2Cat[\s\S]{0,300}22621/i);
});
