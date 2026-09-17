import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const stager = readFileSync('scripts/windows/stage-signed-virtual-driver.ps1', 'utf8');

test('restaging a signed virtual driver preserves lifecycle install state byte-for-byte', () => {
  const removeDestination = stager.indexOf('Remove-Item $destination -Recurse -Force');
  assert.ok(removeDestination >= 0, 'stager must replace the previous destination');

  const captureState = stager.indexOf('[IO.File]::ReadAllBytes($existingInstallStatePath)');
  const restoreState = stager.indexOf("[IO.File]::WriteAllBytes((Join-Path $destination 'virtual-driver-install-state.json'), $existingInstallStateBytes)");
  assert.ok(captureState >= 0 && captureState < removeDestination, 'install state bytes must be captured before destination deletion');
  assert.ok(restoreState > removeDestination, 'captured install state bytes must be restored after destination replacement');
  assert.match(stager, /virtual-driver-install-state\.json/);
});

test('signed virtual-driver staging rejects its source directory and descendants as destination', () => {
  assert.match(stager, /\$destination\s*-eq\s*\$package/i);
  assert.match(stager, /\$destination\.StartsWith\(\$package\s*\+\s*\[IO\.Path\]::DirectorySeparatorChar/i);
  assert.match(stager, /must not be the source package directory or one of its descendants/i);
});
