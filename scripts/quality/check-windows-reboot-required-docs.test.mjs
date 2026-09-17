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
});

test('Tier 3 APO release gate documents non-ready reboot state and resumable uninstall', () => {
  const text = read('docs/release/windows-apo-production-gate.md');
  assert.match(text, /3010/);
  assert.match(text, /restart/i);
  assert.match(text, /bindingReady=false/i);
  assert.match(text, /remaining[\s\S]{0,120}installedInfNames|installedInfNames[\s\S]{0,120}remaining/i);
});
