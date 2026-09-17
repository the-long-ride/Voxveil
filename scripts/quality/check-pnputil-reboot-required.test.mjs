import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

const apoInstaller = read('scripts/windows/install-system-audio-component.ps1');
const systemAudio = read('tauri/app/system_audio.rs');
const uiTypes = read('ui/lib/types.ts');
const uiState = read('ui/app/useVoxveilState.ts');

test('APO install propagates PnPUtil reboot-required without claiming runtime readiness', () => {
  const baseStart = apoInstaller.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const extensionStart = apoInstaller.indexOf('pnputil.exe /add-driver $extensionInf /install');
  const restartAudio = apoInstaller.indexOf('Restart-Service Audiosrv -Force');
  assert.ok(baseStart >= 0 && extensionStart > baseStart && restartAudio > extensionStart);

  for (const [block, exitVariable] of [
    [apoInstaller.slice(baseStart, extensionStart), 'apoPnputilExitCode'],
    [apoInstaller.slice(extensionStart, restartAudio), 'extensionPnputilExitCode'],
  ]) {
    const reboot = block.search(new RegExp(`if\\s*\\(\\s*\\$${exitVariable}\\s*-eq\\s*3010\\s*\\)`, 'i'));
    const hardFailure = block.search(new RegExp(`if\\s*\\(\\s*\\$${exitVariable}\\s*-ne\\s*0\\s*\\)`, 'i'));
    assert.ok(reboot >= 0, `${exitVariable} must recognize PnPUtil reboot-required success`);
    assert.match(block.slice(reboot), /exit\s+3010/i);
    assert.ok(hardFailure > reboot, `${exitVariable} must handle reboot-required before hard failure`);
  }

  assert.match(systemAudio, /enum\s+InstallerLaunchOutcome[\s\S]*RebootRequired/i);
  assert.match(systemAudio, /Some\(3010\)[\s\S]*InstallerLaunchOutcome::RebootRequired/i);
  assert.match(systemAudio, /"reboot-required"/i);
  assert.match(uiTypes, /'reboot-required'/i);
  assert.match(uiState, /result\.outcome\s*===\s*'reboot-required'/i);
  assert.match(uiState, /result\.detail/i);
});
