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
