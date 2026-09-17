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
  const stateWrite = tail.search(/Set-Content\s+\$statePath\s+-Encoding\s+utf8/i);
  const rebootCheck = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-eq\s*3010\s*\)/i);
  const rebootExit = tail.search(/exit\s+3010/i);

  assert.ok(captureExit >= 0, 'PnPUtil delete exit code must be captured');
  assert.ok(exemptFailure > captureExit, 'hard failure must explicitly exempt reboot-required success');
  assert.ok(removeCurrent > exemptFailure, 'deleted package must be removed from recorded ownership only after a successful result');
  assert.ok(checkpoint > removeCurrent, 'remaining package ownership must be checkpointed');
  assert.ok(stateWrite > checkpoint, 'checkpoint must be persisted before reboot handling');
  assert.ok(rebootCheck > stateWrite, 'reboot-required handling must run after the remaining ownership is persisted');
  assert.ok(rebootExit > rebootCheck, '3010 must propagate after the checkpoint');
});
