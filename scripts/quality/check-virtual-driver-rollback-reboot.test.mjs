import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = readFileSync('scripts/windows/install-staged-virtual-driver.ps1', 'utf8');

test('virtual-driver rollback restart records a reboot tombstone before rethrowing the install failure', () => {
  const catchStart = installer.indexOf('catch {');
  assert.ok(catchStart >= 0, 'installer must have a rollback catch block');
  const rollback = installer.slice(catchStart);
  const deletePackage = rollback.indexOf('pnputil.exe /delete-driver $newPublishedInf');
  assert.ok(deletePackage >= 0, 'rollback must delete only the newly added package');
  const deleteTail = rollback.slice(deletePackage);

  const captureExit = deleteTail.search(/\$rollbackDeleteExitCode\s*=\s*\$LASTEXITCODE/i);
  const hardFailure = deleteTail.search(/if\s*\(\s*\$rollbackDeleteExitCode\s*-ne\s*0\s*-and\s*\$rollbackDeleteExitCode\s*-ne\s*3010\s*\)/i);
  const combinedReboot = deleteTail.search(/\$rollbackLifecycleRebootRequired\s*=\s*\$rollbackHelperRebootRequired\s*-or\s*\$rollbackDeleteExitCode\s*-eq\s*3010/i);
  const tombstoneWrite = deleteTail.search(/Write-VirtualDriverInstallState\s+-PublishedInf\s+\$newPublishedInf\s+-PendingReboot\s+\$true\s+-UninstallComplete\s+\$true/i);

  assert.ok(captureExit >= 0, 'rollback delete exit code must be captured');
  assert.ok(hardFailure > captureExit, 'hard rollback deletion failures must be separated from reboot-required success');
  assert.ok(combinedReboot > hardFailure, 'helper and package restart signals must be combined after rollback deletion succeeds');
  assert.ok(tombstoneWrite > combinedReboot, 'successful rollback requiring restart must persist a reboot tombstone');
  assert.match(installer, /\[bool\]\$UninstallComplete\s*=\s*\$false/i);
  assert.match(installer, /uninstallComplete\s*=\s*\$UninstallComplete/i);
});
