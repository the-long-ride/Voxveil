import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Tier 2 signing guide documents PnPUtil reboot-required lifecycle', () => {
  const text = read('docs/release/windows-driver-signing.md');
  assert.match(text, /3010/);
  assert.match(text, /restart/i);
  assert.match(text, /virtual-driver-install-state\.json/i);
  assert.match(text, /pendingReboot/i);
  assert.match(text, /uninstallComplete/i);
  assert.match(text, /boot marker/i);
});

test('Tier 3 APO release gate documents non-ready reboot state and resumable uninstall', () => {
  const text = read('docs/release/windows-apo-production-gate.md');
  assert.match(text, /3010/);
  assert.match(text, /restart/i);
  assert.match(text, /bindingReady=false/i);
  assert.match(text, /remaining[\s\S]{0,120}installedInfNames|installedInfNames[\s\S]{0,120}remaining/i);
});

test('Tier 2 real-machine matrix records install and uninstall restart continuation', () => {
  const text = read('docs/testing/windows-signed-virtual-driver.md');
  assert.match(text, /3010/);
  assert.match(text, /pendingReboot/i);
  assert.match(text, /uninstallComplete/i);
  assert.match(text, /boot marker/i);
  assert.match(text, /restart[\s\S]{0,180}rerun|rerun[\s\S]{0,180}restart/i);
});

test('Tier 3 real-machine matrix records non-ready install and resumable uninstall after restart', () => {
  const text = read('docs/testing/windows-apo-capx-hlk.md');
  assert.match(text, /3010/);
  assert.match(text, /bindingReady=false/i);
  assert.match(text, /remaining[\s\S]{0,160}installedInfNames|installedInfNames[\s\S]{0,160}remaining/i);
});
