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


test('virtual driver installer refuses untracked pre-existing Voxveil packages before devnode mutation', () => {
  const inventory = installer.indexOf('$beforePublishedInfNames = @(Get-VoxveilPublishedInfNames)');
  const ensureDevice = installer.indexOf('& $deviceHelper ensure $inf');
  assert.ok(inventory >= 0 && ensureDevice > inventory, 'Driver Store inventory must be captured before devnode mutation');

  const preflight = installer.slice(inventory, ensureDevice);
  assert.match(preflight, /recordedPublishedInf/i);
  assert.match(preflight, /unexpectedPublishedInfNames/i);
  assert.match(preflight, /\$beforePublishedInfNames/i);
  assert.match(preflight, /-ine\s+\$recordedPublishedInf/i);
  assert.match(preflight, /untracked Voxveil Virtual Audio/i);
  assert.match(preflight, /throw/i);
});

test('virtual driver existing ownership state validates its published INF identity before repair', () => {
  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const inventory = installer.indexOf('$beforePublishedInfNames = @(Get-VoxveilPublishedInfNames)');
  assert.ok(stateLoad >= 0 && inventory > stateLoad);

  const preflight = installer.slice(stateLoad, inventory);
  assert.match(preflight, /publishedInf/i);
  assert.match(preflight, /\^oem\\d\+\\\.inf\$/i);
  assert.match(preflight, /\$recordedPublishedInf\s*=\s*\[string\]\$publishedInfProperty\.Value/i);
  assert.match(preflight, /\$previousInfSha256\s*=\s*\[string\]\$previousState\.infSha256/i);
  assert.match(preflight, /\$previousCatalogSha256\s*=\s*\[string\]\$previousState\.catalogSha256/i);
  assert.match(preflight, /\$previousDriverSha256\s*=\s*\[string\]\$previousState\.driverSha256/i);
  assert.match(preflight, /complete signed-package identity/i);
});
