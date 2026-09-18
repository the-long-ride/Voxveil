import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = readFileSync('scripts/windows/install-staged-virtual-driver.ps1', 'utf8');

test('virtual-driver reboot-required state blocks same-boot reinstall mutation', () => {
  assert.match(installer, /function\s+Get-WindowsBootMarker/i);
  assert.match(installer, /pendingRebootBootMarker/i);

  const ensureDevice = installer.indexOf('& $deviceHelper ensure $inf');
  const sameBootGuard = installer.indexOf('Restart Windows before continuing the Voxveil virtual-driver installation');
  assert.ok(ensureDevice >= 0, 'root devnode ensure mutation must exist');
  assert.ok(sameBootGuard >= 0 && sameBootGuard < ensureDevice, 'same-boot retry must fail before devnode mutation');

  const pnpInstall = installer.indexOf('pnputil.exe /add-driver $inf /install');
  const rebootState = installer.indexOf('Write-VirtualDriverInstallState -PublishedInf $publishedInf -PendingReboot $true');
  assert.ok(pnpInstall >= 0, 'virtual-driver PnP mutation must exist');
  assert.ok(rebootState > pnpInstall, '3010 path must persist pending reboot ownership state');
});


test('virtual-driver install checkpoints missing pending-reboot boot marker before devnode mutation', () => {
  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const ensureDevice = installer.indexOf('& $deviceHelper ensure $inf');
  assert.ok(stateLoad >= 0 && ensureDevice > stateLoad);

  const preflight = installer.slice(stateLoad, ensureDevice);
  assert.match(preflight, /\$previousPendingReboot\s*-and\s*-not\s+\$previousBootMarker/i);
  assert.match(preflight, /pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  assert.match(preflight, /Write-JsonStateAtomically\s+-State\s+\$previousState\s+-Path\s+\$statePath/i);
  assert.match(preflight, /Restart Windows before continuing the Voxveil virtual-driver installation/i);
});
