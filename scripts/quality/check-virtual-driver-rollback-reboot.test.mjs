import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = readFileSync('scripts/windows/install-staged-virtual-driver.ps1', 'utf8');

test('virtual-driver rollback 3010 records a reboot tombstone before rethrowing the install failure', () => {
  const catchStart = installer.indexOf('catch {');
  assert.ok(catchStart >= 0, 'installer must have a rollback catch block');
  const rollback = installer.slice(catchStart);
  const deletePackage = rollback.indexOf('pnputil.exe /delete-driver $newPublishedInf');
  assert.ok(deletePackage >= 0, 'rollback must delete only the newly added package');
  const deleteTail = rollback.slice(deletePackage);

  const captureExit = deleteTail.search(/\$rollbackDeleteExitCode\s*=\s*\$LASTEXITCODE/i);
  const rebootCheck = deleteTail.search(/if\s*\(\s*\$rollbackDeleteExitCode\s*-eq\s*3010\s*\)/i);
  const tombstoneWrite = deleteTail.search(/Write-VirtualDriverInstallState\s+-PublishedInf\s+\$newPublishedInf\s+-PendingReboot\s+\$true\s+-UninstallComplete\s+\$true/i);
  const hardFailure = deleteTail.search(/elseif\s*\(\s*\$rollbackDeleteExitCode\s*-ne\s*0\s*\)/i);

  assert.ok(captureExit >= 0, 'rollback delete exit code must be captured');
  assert.ok(rebootCheck > captureExit, '3010 must be distinguished before generic rollback failure handling');
  assert.ok(tombstoneWrite > rebootCheck, 'successful rollback deletion requiring restart must persist a reboot tombstone');
  assert.ok(hardFailure > tombstoneWrite, 'other nonzero rollback results must remain rollback failures');
  assert.match(installer, /\[bool\]\$UninstallComplete\s*=\s*\$false/i);
  assert.match(installer, /uninstallComplete\s*=\s*\$UninstallComplete/i);
});
