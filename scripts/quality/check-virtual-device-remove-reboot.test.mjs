import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const helper = readFileSync('native/windows/driver/VoxveilVirtualAudioDevice.cpp', 'utf8');
const uninstaller = readFileSync('scripts/windows/uninstall-staged-virtual-driver.ps1', 'utf8');

test('SetupAPI virtual-device removal surfaces restart-required flags', () => {
  assert.match(helper, /SetupDiGetDeviceInstallParamsW/i);
  assert.match(helper, /DI_NEEDREBOOT/i);
  assert.match(helper, /DI_NEEDRESTART/i);
  assert.match(helper, /rebootRequired=/i);
});

test('virtual-driver uninstall folds devnode restart into the existing reboot tombstone', () => {
  assert.match(uninstaller, /\$removeOutput\s*=\s*@\(&\s*\$deviceHelper\s+remove\s+\$deviceInstanceId\)/i);
  assert.match(uninstaller, /Get-HelperValue\s+\$removeOutput\s+'rebootRequired'/i);
  assert.match(uninstaller, /\$devnodeRebootRequired/i);
  assert.match(uninstaller, /\$devnodeRebootRequired\s*-or\s*\$pnputilExitCode\s*-eq\s*3010/i);
  assert.match(uninstaller, /pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  assert.match(uninstaller, /uninstallComplete\s*=\s*\$true/i);
});
