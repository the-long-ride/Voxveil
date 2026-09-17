import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = readFileSync('scripts/windows/install-system-audio-component.ps1', 'utf8');
const uninstaller = readFileSync('scripts/windows/uninstall-system-audio-component.ps1', 'utf8');
const apoRoute = readFileSync('crates/voxveil-windows-audio/src/apo_route.rs', 'utf8');

const modes = ['capx-extension', 'legacy-runtime-interface', 'legacy-reference'];

test('installer, runtime readiness, and uninstaller share stable binding-mode names', () => {
  for (const mode of modes) {
    assert.match(installer, new RegExp(mode));
    assert.match(apoRoute, new RegExp(mode));
  }
  assert.match(uninstaller, /legacy-runtime-interface/);
  assert.doesNotMatch(uninstaller, /bindingMode\s*-eq\s*'runtime-interface'/i);
});

test('legacy runtime attachment is detached before driver package removal', () => {
  assert.match(uninstaller, /detach-effects/);
  assert.match(uninstaller, /bindingPnpInstanceId/);
  assert.match(uninstaller, /topologyInterfacePath/);
  assert.match(uninstaller, /audioInterfacePath/);
});

test('installer records only Voxveil package INF names added by its own invocation', () => {
  assert.match(installer, /beforeInstalledInfNames/);
  assert.match(installer, /afterInstalledInfNames/);
  assert.match(installer, /previousInstalledInfNames/);
  assert.match(installer, /Where-Object\s*\{\s*\$beforeInstalledInfNames\s*-inotcontains\s*\$_\s*\}/s);
});

test('APO ownership inventory reads the Driver Store, including staged but not-yet-bound packages', () => {
  const helperMatch = installer.match(/function\s+Get-VoxveilPublishedInfNames\s*\{([\s\S]*?)\n\}/i);
  assert.ok(helperMatch, 'installer must define Get-VoxveilPublishedInfNames');
  const helper = helperMatch[1];

  assert.match(helper, /Get-WindowsDriver\s+-Online/i);
  assert.match(helper, /ProviderName\s+-ieq\s*'Voxveil'/i);
  assert.match(helper, /OriginalFileName/i);
  assert.match(helper, /GetFileName\s*\(/i);
  assert.match(helper, /VoxveilApo\.inf/i);
  assert.match(helper, /VoxveilApoExtension\.inf/i);
  assert.match(helper, /\.Driver\b/);
  assert.doesNotMatch(helper, /Win32_PnPSignedDriver/i);
});

test('installer snapshots scoped package ownership after each successful PnP package add', () => {
  assert.match(installer, /function\s+Write-InstallStateSnapshot/i);

  const baseInstall = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const extensionInstall = installer.indexOf('pnputil.exe /add-driver $extensionInf /install');
  const restartAudio = installer.indexOf('Restart-Service Audiosrv -Force');

  assert.ok(baseInstall >= 0, 'base APO PnP install must exist');
  assert.ok(extensionInstall > baseInstall, 'Extension INF install must follow the base APO install');
  assert.ok(restartAudio > extensionInstall, 'AudioSrv restart must follow both package installs');
  assert.match(
    installer.slice(baseInstall, extensionInstall),
    /Write-InstallStateSnapshot/,
    'ownership must be persisted after the base APO package succeeds and before Extension install can fail',
  );
  assert.match(
    installer.slice(extensionInstall, restartAudio),
    /Write-InstallStateSnapshot/,
    'ownership must be refreshed after the Extension package succeeds',
  );
});

test('APO uninstaller checkpoints remaining package ownership after each successful delete', () => {
  assert.match(
    uninstaller,
    /\$infNames\s*=\s*@\(\$infNames\s*\|\s*Where-Object\s*\{\s*\$_\s*-ine\s*\$inf\s*\}\)/s,
  );
  assert.match(uninstaller, /\$state\.installedInfNames\s*=\s*@\(\$infNames\)/);

  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const checkpoint = uninstaller.indexOf('$state.installedInfNames = @($infNames)');
  const finalStateRemoval = uninstaller.indexOf('Remove-Item $statePath -Force -ErrorAction SilentlyContinue');
  assert.ok(deleteDriver >= 0, 'scoped APO package deletion must exist');
  assert.ok(checkpoint > deleteDriver, 'remaining ownership must be checkpointed only after a successful deletion');
  assert.ok(finalStateRemoval > checkpoint, 'install state must survive until every recorded package is deleted');
});

test('APO uninstall never falls back to deleting every Voxveil provider package', () => {
  assert.doesNotMatch(
    uninstaller,
    /Get-CimInstance\s+Win32_PnPSignedDriver[\s\S]*DriverProviderName\s+-eq\s*'Voxveil'/i,
  );
  assert.match(uninstaller, /No APO\/Extension package identities were recorded/i);
});

test('manual legacy-reference installs without endpoint identity are not treated as corrupt state', () => {
  assert.match(apoRoute, /endpoint_id:\s*Option<String>/);
  assert.match(apoRoute, /legacy-reference/);
  assert.match(apoRoute, /return\s+Ok\(None\)/);
});

test('raw HardwareId and ReferenceString mode is development/test-sign only', () => {
  assert.match(installer, /ParameterSetName\s+-eq\s*'Manual'/i);
  assert.match(installer, /-not\s+\$TestSign/);
  assert.match(installer, /Manual[^'\r\n]*development|development[^'\r\n]*Manual/i);
});
