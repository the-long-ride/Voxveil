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

test('historical Tier 2 plan marks its undecorated INF guidance as superseded', () => {
  const plan = read('docs/superpowers/plans/2026-09-14-windows-tier2-signed-virtual-driver.md');
  const corrections = read('docs/superpowers/plans/2026-09-14-windows-signed-audio-paths-review-notes.md');

  assert.match(plan, /undecorated[\s\S]{0,240}superseded|superseded[\s\S]{0,240}undecorated/i);
  assert.match(plan, /22621/);
  assert.match(plan, /Tier 1[\s\S]{0,180}Windows 10|Windows 10[\s\S]{0,180}Tier 1/i);
  assert.match(corrections, /Tier 2[\s\S]{0,500}22621/i);
  assert.match(corrections, /undecorated[\s\S]{0,240}superseded|superseded[\s\S]{0,240}undecorated/i);
});
