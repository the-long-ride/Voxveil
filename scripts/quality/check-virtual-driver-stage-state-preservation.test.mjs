import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const stager = readFileSync('scripts/windows/stage-signed-virtual-driver.ps1', 'utf8');

test('restaging a signed virtual driver preserves lifecycle install state', () => {
  const removeDestination = stager.indexOf('Remove-Item $destination -Recurse -Force');
  assert.ok(removeDestination >= 0, 'stager must replace the previous destination');

  const captureState = stager.indexOf('Get-Content $existingInstallStatePath -Raw');
  const restoreState = stager.indexOf("Set-Content (Join-Path $destination 'virtual-driver-install-state.json')");
  assert.ok(captureState >= 0 && captureState < removeDestination, 'install state must be captured before destination deletion');
  assert.ok(restoreState > removeDestination, 'captured install state must be restored after destination replacement');
  assert.match(stager, /virtual-driver-install-state\.json/);
});
