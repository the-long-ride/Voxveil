import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const helper = readFileSync('native/windows/driver/VoxveilVirtualAudioDevice.cpp', 'utf8');
const installer = readFileSync('scripts/windows/install-staged-virtual-driver.ps1', 'utf8');
const uninstaller = readFileSync('scripts/windows/uninstall-staged-virtual-driver.ps1', 'utf8');

test('SetupAPI helper reports restart flags after register/remove class-installer calls', () => {
  assert.match(helper, /bool\s+DeviceInstallNeedsRestart/i);
  assert.match(helper, /SetupDiGetDeviceInstallParamsW/i);
  assert.match(helper, /DI_NEEDREBOOT/i);
  assert.match(helper, /DI_NEEDRESTART/i);
  assert.match(helper, /rebootRequired=/i);

  const register = helper.indexOf('SetupDiCallClassInstaller(DIF_REGISTERDEVICE');
  const remove = helper.indexOf('SetupDiCallClassInstaller(DIF_REMOVE');
  assert.ok(register >= 0 && remove >= 0, 'helper must keep register/remove SetupAPI calls');
  assert.ok(helper.indexOf('DeviceInstallNeedsRestart', register) > register, 'register path must inspect restart flags after SetupAPI success');
  assert.ok(helper.indexOf('DeviceInstallNeedsRestart', remove) > remove, 'remove path must inspect restart flags after SetupAPI success');
});

test('virtual-driver installer aggregates helper restart with PnPUtil restart state', () => {
  assert.match(installer, /Get-HelperValue\s+\$ensureOutput\s+'rebootRequired'/i);
  assert.match(installer, /helperRebootRequired/i);
  assert.match(installer, /\$helperRebootRequired\s*-or\s*\$pnputilExitCode\s*-eq\s*3010/i);
  assert.match(installer, /Write-VirtualDriverInstallState\s+-PublishedInf\s+\$publishedInf\s+-PendingReboot\s+\$true/i);
});

test('virtual-driver install rollback preserves helper restart requirements', () => {
  const catchStart = installer.indexOf('catch {');
  assert.ok(catchStart >= 0, 'installer must keep rollback catch block');
  const rollback = installer.slice(catchStart);
  assert.match(rollback, /Get-HelperValue\s+\$rollbackRemoveOutput\s+'rebootRequired'/i);
  assert.match(rollback, /rollbackHelperRebootRequired/i);
  assert.match(rollback, /\$rollbackHelperRebootRequired\s*-or\s*\$rollbackDeleteExitCode\s*-eq\s*3010/i);
  assert.match(rollback, /Write-VirtualDriverInstallState\s+-PublishedInf\s+\$newPublishedInf\s+-PendingReboot\s+\$true\s+-UninstallComplete\s+\$true/i);
});

test('virtual-driver rollback persists package-less reboot tombstone when only devnode removal requires restart', () => {
  assert.match(installer, /function\s+Write-VirtualDriverRebootTombstone/i);
  assert.match(installer, /pendingReboot\s*=\s*\$true/i);
  assert.match(installer, /pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  assert.match(installer, /uninstallComplete\s*=\s*\$true/i);

  const catchStart = installer.indexOf('catch {');
  assert.ok(catchStart >= 0, 'installer must keep rollback catch block');
  const rollback = installer.slice(catchStart);
  const noPackageBranch = rollback.indexOf('elseif ($rollbackHelperRebootRequired)');
  assert.ok(noPackageBranch >= 0, 'rollback must retain the helper-only reboot branch');
  assert.match(
    rollback.slice(noPackageBranch),
    /Write-VirtualDriverRebootTombstone/i,
    'helper-only reboot must persist a tombstone instead of only warning',
  );
});

test('virtual-driver uninstaller aggregates helper restart with PnPUtil restart tombstone', () => {
  assert.match(uninstaller, /Get-HelperValue\s+\$removeOutput\s+'rebootRequired'/i);
  assert.match(uninstaller, /helperRebootRequired/i);
  assert.match(uninstaller, /\$helperRebootRequired\s*-or\s*\$pnputilExitCode\s*-eq\s*3010/i);
  assert.match(uninstaller, /uninstallComplete\s*=\s*\$true/i);
  assert.match(uninstaller, /pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
});


test('SetupAPI helper exposes a non-mutating exact-device query for reboot tombstone validation', () => {
  assert.match(helper, /int\s+QueryCommand/i);
  assert.match(helper, /SetupDiOpenDeviceInfoW/i);
  assert.match(helper, /HasExactHardwareId/i);
  assert.match(helper, /exists=/i);
  assert.match(helper, /query\s+<device-instance-id>/i);

  const queryBranch = helper.search(/_wcsicmp\(command\.c_str\(\),\s*L"query"\)/i);
  assert.ok(queryBranch >= 0, 'helper CLI must route the query command');
});

test('virtual-driver install and uninstall absence checks use SetupAPI query instead of signed-driver binding visibility', () => {
  const installHelper = installer.match(/function\s+Assert-CompletedUninstallAbsent[^\{]*\{([\s\S]*?)\n\}/i);
  const uninstallHelper = uninstaller.match(/function\s+Assert-CompletedUninstallAbsent[^\{]*\{([\s\S]*?)\n\}/i);
  assert.ok(installHelper && uninstallHelper, 'both lifecycle entrypoints must define completed-uninstall absence validation');

  for (const body of [installHelper[1], uninstallHelper[1]]) {
    assert.match(body, /query\s+\$deviceInstanceId/i);
    assert.match(body, /Get-HelperValue[\s\S]*'exists'/i);
    assert.doesNotMatch(body, /Win32_PnPSignedDriver/i);
  }
});
