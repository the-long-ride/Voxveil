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
  const completedAbsenceCheck = uninstaller.indexOf('Assert-CompletedUninstallAbsent $state', completedGuard);
  const completedStateRemoval = uninstaller.indexOf('Remove-Item $statePath -Force', completedGuard);
  assert.ok(pendingGuard >= 0 && pendingGuard < completedGuard, 'any pending same-boot reboot must block uninstall mutation, including a pending install');
  assert.ok(completedGuard >= 0 && completedGuard < identityCheck, 'completed-uninstall reboot tombstone must be handled before stale package identity checks');
  assert.ok(completedAbsenceCheck > completedGuard && completedAbsenceCheck < completedStateRemoval, 'post-reboot tombstone cleanup must prove the recorded removal is complete before discarding state');
  assert.ok(completedStateRemoval > completedAbsenceCheck && completedStateRemoval < identityCheck, 'post-reboot tombstone cleanup must happen only after absence validation');

  const ensureDevice = installer.indexOf('& $deviceHelper ensure $inf');
  const sameBootGuard = installer.indexOf('Restart Windows before continuing the Voxveil virtual-driver installation.');
  assert.ok(ensureDevice >= 0, 'root devnode ensure mutation must exist');
  assert.ok(sameBootGuard >= 0 && sameBootGuard < ensureDevice, 'same-boot reinstall must fail before devnode mutation');
});

test('completed uninstall absence check is scoped and supports package-less rollback tombstones', () => {
  assert.match(uninstaller, /function\s+Assert-CompletedUninstallAbsent/i);
  const helperStart = uninstaller.search(/function\s+Assert-CompletedUninstallAbsent/i);
  const helperEnd = uninstaller.indexOf('function Get-WindowsBootMarker', helperStart);
  assert.ok(helperStart >= 0 && helperEnd > helperStart, 'absence helper must be defined before boot-marker logic');
  const helper = uninstaller.slice(helperStart, helperEnd);

  assert.match(helper, /Get-WindowsDriver\s+-Online/i);
  assert.match(helper, /query\s+\$deviceInstanceId/i);
  assert.match(helper, /Get-HelperValue[\s\S]*'exists'/i);
  assert.doesNotMatch(helper, /Win32_PnPSignedDriver/i);
  assert.match(helper, /VoxveilVirtualAudio\.inf/i);
  assert.match(helper, /return/i, 'package-less reboot tombstones must be clearable after the boot changes');
});


test('virtual driver install proves completed uninstall absence after reboot before new mutation', () => {
  assert.match(installer, /function\s+Assert-CompletedUninstallAbsent/i);

  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const sameBootGuard = installer.indexOf('Restart Windows before continuing the Voxveil virtual-driver installation.');
  const absenceCheck = installer.indexOf('Assert-CompletedUninstallAbsent $previousState', sameBootGuard);
  const ensureDevice = installer.indexOf('& $deviceHelper ensure $inf');

  assert.ok(stateLoad >= 0, 'installer must load persisted lifecycle state');
  assert.ok(sameBootGuard > stateLoad, 'same-boot restart guard must run after state load');
  assert.ok(absenceCheck > sameBootGuard, 'post-reboot absence proof must run after the same-boot guard');
  assert.ok(ensureDevice > absenceCheck, 'completed-uninstall absence must be proved before creating or reusing the devnode');
});


test('virtual driver uninstall refreshes exact Driver Store presence before propagating hard delete failure', () => {
  assert.match(uninstaller, /function\s+Test-RecordedVirtualDriverPackagePresent/i);
  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $publishedInf');
  assert.ok(deleteDriver >= 0, 'uninstaller must delete only the recorded published INF');
  const tail = uninstaller.slice(deleteDriver);

  const captureExit = tail.search(/\$pnputilExitCode\s*=\s*\$LASTEXITCODE/i);
  const refreshPresence = tail.search(/\$packageStillPresent\s*=\s*Test-RecordedVirtualDriverPackagePresent\s+\$publishedInf/i);
  const hardFailure = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-ne\s*0\s*-and\s*\$pnputilExitCode\s*-ne\s*3010\s*\)/i);
  assert.ok(captureExit >= 0, 'PnPUtil delete exit code must be captured');
  assert.ok(refreshPresence > captureExit, 'Driver Store presence must be refreshed after PnPUtil returns');
  assert.ok(hardFailure > refreshPresence, 'hard delete failure must be interpreted only after ownership refresh');

  const hardTail = tail.slice(hardFailure);
  const lifecycle = hardTail.indexOf('$lifecycleRebootRequired');
  assert.ok(lifecycle > 0, 'hard failure recovery must finish before successful/reboot-required handling');
  const hardBlock = hardTail.slice(0, lifecycle);
  assert.match(hardBlock, /if\s*\(\s*-not\s+\$packageStillPresent\s*\)/i);
  assert.match(hardBlock, /Remove-Item\s+\$statePath\s+-Force/i);
  assert.match(hardBlock, /uninstallComplete\s*=\s*\$true/i);
});

test('virtual driver uninstall fails closed when PnPUtil reports success but package remains', () => {
  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $publishedInf');
  const tail = uninstaller.slice(deleteDriver);
  const refreshPresence = tail.search(/\$packageStillPresent\s*=\s*Test-RecordedVirtualDriverPackagePresent\s+\$publishedInf/i);
  const staleSuccessGuard = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-eq\s*0\s*-and\s*\$packageStillPresent\s*\)/i);
  const lifecycle = tail.search(/\$lifecycleRebootRequired\s*=\s*\$helperRebootRequired\s*-or\s*\$pnputilExitCode\s*-eq\s*3010/i);

  assert.ok(refreshPresence >= 0, 'post-delete Driver Store presence refresh must exist');
  assert.ok(staleSuccessGuard > refreshPresence, 'successful exit must prove the package disappeared');
  assert.ok(lifecycle > staleSuccessGuard, 'restart/success state must be committed only after the successful-delete absence proof');
});
