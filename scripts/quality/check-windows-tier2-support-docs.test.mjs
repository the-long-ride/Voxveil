import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

const WINDOWS_DRIVER_SAMPLES_REVISION = '67d81f217bc01edf7a4320e4911c11065635acfa';
const SYSVAD_TREE_SHA = '6fa502f5bfb3de1395a6c9ffe71e322fd9e28926';

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

test('driver signing guide identifies the exact-SHA Manual Build submission artifact', () => {
  const text = read('docs/release/windows-driver-signing.md');
  assert.match(text, /Manual Build/i);
  assert.match(text, /Voxveil-windows-driver-submission-\$\{\{\s*github\.sha\s*\}\}/);
  assert.match(text, /exact[ -]SHA|exact commit/i);
});

test('driver signing guide requires uninstall before changing the recorded signed package', () => {
  const text = read('docs/release/windows-driver-signing.md');
  assert.match(text, /uninstall[\s\S]{0,220}different signed package|different signed package[\s\S]{0,220}uninstall/i);
  assert.match(text, /same-package[\s\S]{0,180}repair|repair[\s\S]{0,180}same-package/i);
});

test('signed-driver validation matrix records pinned source provenance', () => {
  const text = read('docs/testing/windows-signed-virtual-driver.md');
  assert.ok(
    text.includes(WINDOWS_DRIVER_SAMPLES_REVISION),
    'validation matrix must record the pinned Windows Driver Samples revision',
  );
  assert.ok(text.includes(SYSVAD_TREE_SHA), 'validation matrix must record the pinned SysVAD tree SHA');
});

test('authoritative Tier 2 corrections supersede undecorated INF guidance', () => {
  const corrections = read('docs/superpowers/plans/2026-09-14-windows-signed-audio-paths-review-notes.md');

  assert.match(corrections, /### 5\. Preserve the pinned SysVAD Windows 11 build-22621 applicability floor/i);
  assert.match(corrections, /original Tier 2 Task 4[\s\S]{0,220}undecorated/i);
  assert.match(corrections, /undecorated[\s\S]{0,300}superseded|superseded[\s\S]{0,300}undecorated/i);
  assert.match(corrections, /Windows 10[\s\S]{0,300}Tier 1|Tier 1[\s\S]{0,300}Windows 10/i);
  assert.match(corrections, /Inf2Cat[\s\S]{0,500}22621|22621[\s\S]{0,500}Inf2Cat/i);
});
