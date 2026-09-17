import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const uninstaller = readFileSync('scripts/windows/uninstall-staged-virtual-driver.ps1', 'utf8');
const installer = readFileSync('scripts/windows/install-staged-virtual-driver.ps1', 'utf8');

test('virtual driver uninstall treats package or devnode restart as successful removal requiring reboot', () => {
  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $publishedInf');
  assert.ok(deleteDriver >= 0, 'uninstaller must delete only the recorded published INF');
  const tail = uninstaller.slice(deleteDriver);

  const captureExit = tail.search(/\$pnputilExitCode\s*=\s*\$LASTEXITCODE/i);
  const hardFailure = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-ne\s*0\s*-and\s*\$pnputilExitCode\s*-ne\s*3010\s*\)/i);
  const combinedReboot = tail.search(/\$lifecycleRebootRequired\s*=\s*\$helperRebootRequired\s*-or\s*\$pnputilExitCode\s*-eq\s*3010/i);

  assert.ok(captureExit >= 0, 'PnPUtil delete exit code must be captured');
  assert.ok(hardFailure > captureExit, 'hard package failures must be separated from reboot-required success');
  assert.ok(combinedReboot > hardFailure, 'devnode and package reboot signals must be combined after package result validation');

  const rebootTail = tail.slice(combinedReboot);
  const tombstone = rebootTail.search(/uninstallComplete\s*=\s*\$true/i);
  const stateWrite = rebootTail.search(/Set-Content\s+\$statePath\s+-Encoding\s+utf8/i);
  const rebootExit = rebootTail.search(/exit\s+3010/i);
  assert.ok(tombstone >= 0, 'successful removal requiring restart must checkpoint an uninstall-complete tombstone');
  assert.ok(stateWrite > tombstone, 'reboot tombstone must be persisted before restart propagation');
  assert.ok(rebootExit > stateWrite, 'reboot-required exit must follow tombstone persistence');
});

test('virtual driver uninstall 3010 blocks same-boot reinstall and clears tombstone only after reboot', () => {
  assert.match(uninstaller, /function\s+Get-WindowsBootMarker/i);
  assert.match(uninstaller, /pendingReboot\s*=\s*\$true/i);
  assert.match(uninstaller, /pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  assert.match(uninstaller, /uninstallComplete\s*=\s*\$true/i);

  const identityCheck = uninstaller.indexOf('Assert-PublishedInfIdentity $publishedInf $deviceInstanceId');
  const pendingGuard = uninstaller.indexOf('if ($pendingReboot -and $pendingBootMarker -and $pendingBootMarker -eq $currentBootMarker)');
  const completedGuard = uninstaller.indexOf('if ($uninstallComplete)');
  const completedStateRemoval = uninstaller.indexOf('Remove-Item $statePath -Force', completedGuard);
  assert.ok(pendingGuard >= 0 && pendingGuard < completedGuard, 'any pending same-boot reboot must block uninstall mutation, including a pending install');
  assert.ok(completedGuard >= 0 && completedGuard < identityCheck, 'completed-uninstall reboot tombstone must be handled before stale package identity checks');
  assert.ok(completedStateRemoval > completedGuard && completedStateRemoval < identityCheck, 'post-reboot tombstone cleanup must happen before package identity revalidation');

  const ensureDevice = installer.indexOf('& $deviceHelper ensure $inf');
  const sameBootGuard = installer.indexOf('Restart Windows before continuing the Voxveil virtual-driver installation.');
  assert.ok(ensureDevice >= 0, 'root devnode ensure mutation must exist');
  assert.ok(sameBootGuard >= 0 && sameBootGuard < ensureDevice, 'same-boot reinstall must fail before devnode mutation');
});
