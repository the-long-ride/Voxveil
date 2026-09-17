import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const text = readFileSync('scripts/windows/uninstall-staged-virtual-driver.ps1', 'utf8');

test('virtual driver uninstall treats PnPUtil 3010 as successful deletion requiring reboot', () => {
  const deleteDriver = text.indexOf('pnputil.exe /delete-driver $publishedInf');
  assert.ok(deleteDriver >= 0, 'uninstaller must delete only the recorded published INF');
  const tail = text.slice(deleteDriver);

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
