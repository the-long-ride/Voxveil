import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = readFileSync('scripts/windows/install-system-audio-component.ps1', 'utf8');

test('APO reboot-required state blocks another install mutation until Windows actually restarts', () => {
  assert.match(installer, /function\s+Get-WindowsBootMarker/i);
  assert.match(installer, /pendingReboot\s*=\s*\$PendingReboot/i);
  assert.match(installer, /pendingRebootBootMarker/i);

  const firstPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const sameBootGuard = installer.indexOf('Restart Windows before continuing the Voxveil system-audio installation');
  assert.ok(firstPnp >= 0, 'base APO PnP mutation must exist');
  assert.ok(sameBootGuard >= 0 && sameBootGuard < firstPnp, 'same-boot retry must fail before any PnP mutation');

  const baseInstall = installer.slice(firstPnp, installer.indexOf('pnputil.exe /add-driver $extensionInf /install'));
  const extensionInstall = installer.slice(
    installer.indexOf('pnputil.exe /add-driver $extensionInf /install'),
    installer.indexOf('Restart-Service Audiosrv -Force'),
  );
  assert.match(baseInstall, /Write-InstallStateSnapshot\s+-PendingReboot\s+\$true/i);
  assert.match(extensionInstall, /Write-InstallStateSnapshot\s+-PendingReboot\s+\$true/i);
});


test('APO install checkpoints missing pending-reboot boot marker before mutation', () => {
  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const firstPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  assert.ok(stateLoad >= 0 && firstPnp > stateLoad);

  const preflight = installer.slice(stateLoad, firstPnp);
  assert.match(preflight, /\$previousPendingReboot\s*-eq\s*\$true\s*-and\s*-not\s+\$previousBootMarker/i);
  assert.match(preflight, /pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  assert.match(preflight, /Write-JsonStateAtomically\s+-State\s+\$previousState\s+-Path\s+\$statePath/i);
  assert.match(preflight, /Restart Windows before continuing the Voxveil system-audio installation/i);
});
