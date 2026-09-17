import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = readFileSync('scripts/windows/install-staged-virtual-driver.ps1', 'utf8');

test('virtual driver installer refuses a different signed package before overwriting existing ownership state', () => {
  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const ensureDevice = installer.indexOf('& $deviceHelper ensure $inf');
  assert.ok(stateLoad >= 0 && ensureDevice > stateLoad, 'existing lifecycle state must be inspected before devnode mutation');

  const guardText = 'Uninstall the currently recorded Voxveil Virtual Audio package before installing a different signed package.';
  const packageGuard = installer.indexOf(guardText, stateLoad);
  assert.ok(packageGuard > stateLoad && packageGuard < ensureDevice, 'different-package install must fail before devnode or PnP mutation');

  const preflight = installer.slice(stateLoad, ensureDevice);
  assert.match(preflight, /uninstallComplete/i);
  assert.match(preflight, /infSha256/i);
  assert.match(preflight, /catalogSha256/i);
  assert.match(preflight, /driverSha256/i);
  assert.match(preflight, /\$verification\.infSha256/i);
  assert.match(preflight, /\$verification\.catalogSha256/i);
  assert.match(preflight, /\$verification\.driverSha256/i);
});
