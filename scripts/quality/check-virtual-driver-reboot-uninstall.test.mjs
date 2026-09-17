import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const uninstaller = readFileSync('scripts/windows/uninstall-staged-virtual-driver.ps1', 'utf8');
const installer = readFileSync('scripts/windows/install-staged-virtual-driver.ps1', 'utf8');

test('virtual driver uninstall treats PnPUtil 3010 as successful deletion requiring reboot', () => {
  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $publishedInf');
  assert.ok(deleteDriver >= 0, 'uninstaller must delete only the recorded published INF');
  const tail = uninstaller.slice(deleteDriver);

  const captureExit = tail.search(/\$pnputilExitCode\s*=\s*\$LASTEXITCODE/i);
  const rebootCheck = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-eq\s*3010\s*\)/i);
  const stateRemoval = tail.search(/Remove-Item\s+\$statePath\s+-Force/i);
  const rebootExit = tail.search(/exit\s+3010/i);
  const hardFailure = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-ne\s*0\s*\)/i);

  assert.ok(captureExit >= 0, 'PnPUtil delete exit code must be captured');
  assert.ok(rebootCheck > captureExit, 'reboot-required success must be handled after the delete result is captured');
  assert.ok(stateRemoval > rebootCheck, 'stale install-state ownership must be removed after successful deletion');
  assert.ok(rebootExit > stateRemoval, 'reboot-required exit must follow state removal');
  assert.ok(hardFailure > rebootExit, 'generic failure handling must not classify 3010 as a failure');
});

test('virtual driver uninstall 3010 blocks same-boot reinstall until Windows actually restarts', () => {
  assert.match(uninstaller, /function\s+Get-WindowsBootMarker/i);
  assert.match(uninstaller, /virtual-driver-reboot-state\.json/i);
  assert.match(uninstaller, /pendingRebootBootMarker/i);
  assert.match(installer, /virtual-driver-reboot-state\.json/i);
  assert.match(installer, /pendingRebootBootMarker/i);

  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $publishedInf');
  const rebootBlock = uninstaller.slice(deleteDriver, uninstaller.indexOf('if ($pnputilExitCode -ne 0)', deleteDriver));
  const stateRemoval = rebootBlock.indexOf('Remove-Item $statePath -Force');
  const markerWrite = rebootBlock.indexOf('Set-Content $rebootStatePath -Encoding utf8');
  const rebootExit = rebootBlock.indexOf('exit 3010');
  assert.ok(stateRemoval >= 0, 'successful deletion must still clear stale install ownership');
  assert.ok(markerWrite > stateRemoval, 'restart marker must be persisted after stale install ownership is removed');
  assert.ok(rebootExit > markerWrite, 'restart-required exit must follow reboot marker persistence');

  const ensureDevice = installer.indexOf('& $deviceHelper ensure $inf');
  const sameBootGuard = installer.indexOf('Restart Windows before reinstalling Voxveil Virtual Audio');
  assert.ok(ensureDevice >= 0, 'root devnode ensure mutation must exist');
  assert.ok(sameBootGuard >= 0 && sameBootGuard < ensureDevice, 'same-boot reinstall must fail before devnode mutation');
});
