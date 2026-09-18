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

test('APO PnP failures refresh scoped ownership before throwing so staged packages remain recoverable', () => {
  const baseInstall = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const extensionInstall = installer.indexOf('pnputil.exe /add-driver $extensionInf /install');
  const restartAudio = installer.indexOf('Restart-Service Audiosrv -Force');
  assert.ok(baseInstall >= 0 && extensionInstall > baseInstall && restartAudio > extensionInstall);

  const baseBlock = installer.slice(baseInstall, extensionInstall);
  const baseCapture = baseBlock.search(/\$apoPnputilExitCode\s*=\s*\$LASTEXITCODE/i);
  const baseSnapshot = baseBlock.search(/Write-InstallStateSnapshot/);
  const baseFailure = baseBlock.search(/if\s*\(\s*\$apoPnputilExitCode\s*-ne\s*0\s*\)/i);
  assert.ok(baseCapture >= 0, 'base APO PnPUtil exit code must be captured');
  assert.ok(baseSnapshot > baseCapture, 'base APO ownership snapshot must run after PnPUtil returns');
  assert.ok(baseFailure > baseSnapshot, 'base APO PnP failure must be thrown only after ownership is persisted');

  const extensionBlock = installer.slice(extensionInstall, restartAudio);
  const extensionCapture = extensionBlock.search(/\$extensionPnputilExitCode\s*=\s*\$LASTEXITCODE/i);
  const extensionSnapshot = extensionBlock.search(/Write-InstallStateSnapshot/);
  const extensionFailure = extensionBlock.search(/if\s*\(\s*\$extensionPnputilExitCode\s*-ne\s*0\s*\)/i);
  assert.ok(extensionCapture >= 0, 'Extension PnPUtil exit code must be captured');
  assert.ok(extensionSnapshot > extensionCapture, 'Extension ownership snapshot must run after PnPUtil returns');
  assert.ok(extensionFailure > extensionSnapshot, 'Extension PnP failure must be thrown only after ownership is persisted');
});

test('partial APO ownership snapshots stay non-ready until AudioDG load verification succeeds', () => {
  assert.match(
    installer,
    /function\s+Write-InstallStateSnapshot\s*\(\s*\[bool\]\$BindingReady\s*=\s*\$false\s*\)/i,
  );
  assert.match(installer, /bindingReady\s*=\s*\$BindingReady/);
  assert.match(apoRoute, /binding_ready:\s*Option<bool>/);
  assert.match(apoRoute, /binding_ready\s*==\s*Some\(false\)/);

  const statusCheck = installer.indexOf('if (Test-Path $control)');
  const loadedVerification = installer.indexOf("$status -notmatch 'loaded=[1-9][0-9]*'", statusCheck);
  const readySnapshot = installer.indexOf('Write-InstallStateSnapshot -BindingReady $true', statusCheck);
  assert.ok(statusCheck >= 0 && loadedVerification > statusCheck, 'installer must verify real AudioDG load');
  assert.ok(readySnapshot > loadedVerification, 'binding readiness must be committed only after loaded= verification passes');
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

test('APO uninstaller revalidates recorded Driver Store identity before deletion', () => {
  const helperMatch = uninstaller.match(/function\s+Assert-RecordedApoInfIdentity[^\{]*\{([\s\S]*?)\n\}/i);
  assert.ok(helperMatch, 'uninstaller must define Assert-RecordedApoInfIdentity');
  const helper = helperMatch[1];

  assert.match(helper, /Get-WindowsDriver\s+-Online/i);
  assert.match(helper, /\.Driver\s+-ieq\s+\$PublishedInf/i);
  assert.match(helper, /ProviderName/i);
  assert.match(helper, /OriginalFileName/i);
  assert.match(helper, /GetFileName\s*\(/i);
  assert.match(helper, /VoxveilApo\.inf/i);
  assert.match(helper, /VoxveilApoExtension\.inf/i);

  const identityCheck = uninstaller.indexOf('Assert-RecordedApoInfIdentity $inf');
  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  assert.ok(identityCheck >= 0, 'recorded INF identity must be checked');
  assert.ok(deleteDriver > identityCheck, 'identity must be revalidated before deleting the package');
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


test('APO installer proves pending removed INF absence after reboot before new PnP mutation', () => {
  assert.match(installer, /function\s+Assert-PendingRemovedApoInfAbsent/i);
  assert.match(installer, /pendingRemovedInfName/i);

  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const sameBootGuard = installer.indexOf('Restart Windows before continuing the Voxveil system-audio installation.');
  const absenceCheck = installer.indexOf('Assert-PendingRemovedApoInfAbsent $previousPendingRemovedInfName', sameBootGuard);
  const baseInstall = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");

  assert.ok(stateLoad >= 0, 'installer must load persisted APO lifecycle state');
  assert.ok(sameBootGuard > stateLoad, 'same-boot restart guard must run after state load');
  assert.ok(absenceCheck > sameBootGuard, 'post-reboot absence proof must run after the same-boot guard');
  assert.ok(baseInstall > absenceCheck, 'pending removed APO identity must be proved absent before new PnP mutation');

  const helperStart = installer.search(/function\s+Assert-PendingRemovedApoInfAbsent/i);
  const helperEnd = installer.indexOf('function Get-WindowsBootMarker', helperStart);
  assert.ok(helperStart >= 0 && helperEnd > helperStart, 'APO absence helper must be defined before boot marker logic');
  const helper = installer.slice(helperStart, helperEnd);
  assert.match(helper, /Get-WindowsDriver\s+-Online/i);
  assert.match(helper, /VoxveilApo\.inf|VoxveilApoExtension\.inf/i);
});


test('APO uninstaller refreshes Driver Store ownership before propagating hard delete failures', () => {
  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  assert.ok(deleteDriver >= 0, 'scoped APO package deletion must exist');
  const tail = uninstaller.slice(deleteDriver);

  const captureExit = tail.search(/\$pnputilExitCode\s*=\s*\$LASTEXITCODE/i);
  const refreshPresence = tail.search(/\$packageStillPresent\s*=\s*Test-RecordedApoInfPresent\s+\$inf/i);
  const hardFailure = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-ne\s*0\s*-and\s*\$pnputilExitCode\s*-ne\s*3010\s*\)/i);
  assert.ok(captureExit >= 0, 'PnPUtil delete exit code must be captured');
  assert.ok(refreshPresence > captureExit, 'Driver Store ownership must be refreshed after PnPUtil returns');
  assert.ok(hardFailure > refreshPresence, 'hard delete failure must be interpreted only after ownership refresh');

  const staleSuccessGuard = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-eq\s*0\s*-and\s*\$packageStillPresent\s*\)/i);
  assert.ok(staleSuccessGuard > hardFailure, 'hard failure recovery must finish before successful-delete absence validation');
  const hardBlock = tail.slice(hardFailure, staleSuccessGuard);
  assert.match(hardBlock, /if\s*\(\s*-not\s+\$packageStillPresent\s*\)/i);
  assert.match(hardBlock, /\$state\.installedInfNames\s*=\s*@\(\$infNames\)/i);
  assert.match(hardBlock, /Set-Content\s+\$statePath\s+-Encoding\s+utf8/i);
});

test('APO uninstaller fails closed when PnPUtil reports success but the recorded package remains', () => {
  assert.match(uninstaller, /function\s+Test-RecordedApoInfPresent/i);
  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const tail = uninstaller.slice(deleteDriver);
  const refreshPresence = tail.search(/\$packageStillPresent\s*=\s*Test-RecordedApoInfPresent\s+\$inf/i);
  const staleSuccessGuard = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-eq\s*0\s*-and\s*\$packageStillPresent\s*\)/i);
  const ownershipDrop = tail.indexOf('$infNames = @($infNames | Where-Object', staleSuccessGuard);

  assert.ok(refreshPresence >= 0, 'post-delete Driver Store presence refresh must exist');
  assert.ok(staleSuccessGuard > refreshPresence, 'successful exit must still prove the package disappeared');
  assert.ok(ownershipDrop > staleSuccessGuard, 'normal ownership must be dropped only after the successful-delete absence proof');
});


test('APO installer refuses untracked pre-existing Voxveil packages before PnP mutation', () => {
  const inventory = installer.indexOf('$beforeInstalledInfNames = @(Get-VoxveilPublishedInfNames)');
  const baseInstall = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  assert.ok(inventory >= 0 && baseInstall > inventory, 'Driver Store inventory must be checked before APO PnP mutation');

  const preflight = installer.slice(inventory, baseInstall);
  assert.match(preflight, /unexpectedInstalledInfNames/i);
  assert.match(preflight, /previousInstalledInfNames/i);
  assert.match(preflight, /-inotcontains\s+\$_/i);
  assert.match(preflight, /untracked Voxveil APO\/Extension package/i);
  assert.match(preflight, /throw/i);
});


test('APO installer refuses a second different managed endpoint before PnP mutation', () => {
  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const descriptorResolve = installer.indexOf('$binding = Resolve-EndpointDescriptor $EndpointDescriptor $root');
  const baseInstall = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  assert.ok(stateLoad >= 0 && descriptorResolve > stateLoad && baseInstall > descriptorResolve);

  const preflight = installer.slice(stateLoad, baseInstall);
  assert.match(preflight, /previousReadyEndpointId/i);
  assert.match(preflight, /bindingReady/i);
  assert.match(preflight, /selectedEndpointId/i);
  assert.match(preflight, /-ine\s+\$previousReadyEndpointId/i);
  assert.match(preflight, /Uninstall the currently managed Voxveil APO endpoint/i);
});
