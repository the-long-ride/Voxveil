import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const text = readFileSync('scripts/windows/uninstall-system-audio-component.ps1', 'utf8');

test('APO uninstall checkpoints successful 3010 deletion before requiring reboot', () => {
  const deleteDriver = text.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  assert.ok(deleteDriver >= 0, 'uninstaller must delete only a recorded APO/Extension package');
  const tail = text.slice(deleteDriver);

  const captureExit = tail.search(/\$pnputilExitCode\s*=\s*\$LASTEXITCODE/i);
  const exemptFailure = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-ne\s*0\s*-and\s*\$pnputilExitCode\s*-ne\s*3010\s*\)/i);
  const removeCurrent = tail.search(/\$infNames\s*=\s*@\(\$infNames\s*\|\s*Where-Object\s*\{\s*\$_\s*-ine\s*\$inf\s*\}\)/i);
  const checkpoint = tail.search(/\$state\.installedInfNames\s*=\s*@\(\$infNames\)/i);
  const pendingRemoved = tail.search(/\$state\.pendingRemovedInfName\s*=\s*\$inf/i);
  const stateWriteRelative = tail.slice(pendingRemoved).search(/Write-JsonStateAtomically\s+-State\s+\$state\s+-Path\s+\$statePath/i);
  const stateWrite = stateWriteRelative >= 0 ? pendingRemoved + stateWriteRelative : -1;
  const rebootCheckRelative = tail.slice(stateWrite).search(/if\s*\(\s*\$pnputilExitCode\s*-eq\s*3010\s*\)/i);
  const rebootCheck = rebootCheckRelative >= 0 ? stateWrite + rebootCheckRelative : -1;
  const rebootExit = tail.search(/exit\s+3010/i);

  assert.ok(captureExit >= 0, 'PnPUtil delete exit code must be captured');
  assert.ok(exemptFailure > captureExit, 'hard failure must explicitly exempt reboot-required success');
  assert.ok(removeCurrent > exemptFailure, 'deleted package must be removed from recorded ownership only after a successful result');
  assert.ok(checkpoint > removeCurrent, 'remaining package ownership must be checkpointed');
  assert.ok(pendingRemoved > checkpoint, '3010 cleanup must retain the just-removed INF identity for post-reboot absence proof');
  assert.ok(stateWrite > pendingRemoved, 'checkpoint and pending removed identity must be persisted before reboot handling');
  assert.ok(rebootCheck > stateWrite, 'reboot-required handling must run after the remaining ownership is persisted');
  assert.ok(rebootExit > rebootCheck, '3010 must propagate after the checkpoint');
});

test('APO uninstall blocks same-boot continuation after reboot-required deletion', () => {
  assert.match(text, /function\s+Get-WindowsBootMarker/i);
  assert.match(text, /pendingRebootBootMarker/i);

  const detachEffects = text.indexOf('& $control detach-effects');
  const deleteDriver = text.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const sameBootGuard = text.indexOf('Restart Windows before continuing Voxveil APO package cleanup');
  assert.ok(deleteDriver >= 0, 'recorded package delete must exist');
  assert.ok(sameBootGuard >= 0 && sameBootGuard < deleteDriver, 'same-boot guard must run before package deletion');
  if (detachEffects >= 0) {
    assert.ok(sameBootGuard < detachEffects, 'same-boot guard must run before legacy FX detach mutation');
  }

  const tail = text.slice(deleteDriver);
  const markPending = tail.search(/\$state\.pendingReboot\s*=\s*\$true/i);
  const markBoot = tail.search(/\$state\.pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  const stateWriteRelative = tail.slice(markBoot).search(/Write-JsonStateAtomically\s+-State\s+\$state\s+-Path\s+\$statePath/i);
  const stateWrite = stateWriteRelative >= 0 ? markBoot + stateWriteRelative : -1;
  const rebootExit = tail.search(/exit\s+3010/i);
  assert.ok(markPending >= 0, '3010 cleanup state must remain pending reboot');
  assert.ok(markBoot > markPending, '3010 cleanup state must record the current boot marker');
  assert.ok(stateWrite > markBoot, 'pending reboot marker must be persisted');
  assert.ok(rebootExit > stateWrite, 'restart-required exit must follow state persistence');
});


test('APO uninstall proves pending removed package absence after reboot before discarding its identity', () => {
  assert.match(text, /function\s+Assert-PendingRemovedApoInfAbsent/i);
  assert.match(text, /pendingRemovedInfName/i);

  const stateLoad = text.indexOf('$state = Get-Content $statePath -Raw | ConvertFrom-Json');
  const sameBootGuard = text.indexOf('Restart Windows before continuing Voxveil APO package cleanup');
  const absenceCheck = text.indexOf('Assert-PendingRemovedApoInfAbsent $pendingRemovedInfName', sameBootGuard);
  const clearIdentity = text.indexOf('$state.pendingRemovedInfName = $null', absenceCheck);
  const detachEffects = text.indexOf('& $control detach-effects');
  const deleteDriver = text.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');

  assert.ok(stateLoad >= 0, 'install state must be loaded');
  assert.ok(sameBootGuard > stateLoad, 'same-boot reboot guard must run after state load');
  assert.ok(absenceCheck > sameBootGuard, 'post-reboot absence proof must run only after the same-boot guard');
  assert.ok(clearIdentity > absenceCheck, 'pending removed identity must be cleared only after absence proof');
  if (detachEffects >= 0) {
    assert.ok(clearIdentity < detachEffects, 'absence proof must complete before legacy FX detach mutation');
  }
  assert.ok(clearIdentity < deleteDriver, 'absence proof must complete before deleting remaining APO packages');

  const helperStart = text.search(/function\s+Assert-PendingRemovedApoInfAbsent/i);
  const helperEnd = text.indexOf('function Get-WindowsBootMarker', helperStart);
  assert.ok(helperStart >= 0 && helperEnd > helperStart, 'absence helper must be defined before boot marker logic');
  const helper = text.slice(helperStart, helperEnd);
  assert.match(helper, /Get-WindowsDriver\s+-Online/i);
  assert.match(helper, /VoxveilApo\.inf|VoxveilApoExtension\.inf/i);
});


test('APO uninstall checkpoints missing pending-reboot boot marker before cleanup mutation', () => {
  const stateLoad = text.indexOf('$state = Get-Content $statePath -Raw | ConvertFrom-Json');
  const detachEffects = text.indexOf('& $control detach-effects');
  const deleteDriver = text.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const firstMutation = detachEffects >= 0 ? Math.min(detachEffects, deleteDriver) : deleteDriver;
  assert.ok(stateLoad >= 0 && firstMutation > stateLoad);

  const preflight = text.slice(stateLoad, firstMutation);
  assert.match(preflight, /\$pendingReboot\s*-and\s*-not\s+\$pendingBootMarker/i);
  assert.match(preflight, /pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  assert.match(preflight, /Write-JsonStateAtomically\s+-State\s+\$state\s+-Path\s+\$statePath/i);
  assert.match(preflight, /Restart Windows before continuing Voxveil APO package cleanup/i);
});
