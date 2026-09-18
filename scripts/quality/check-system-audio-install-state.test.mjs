import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = readFileSync('scripts/windows/install-system-audio-component.ps1', 'utf8');
const uninstaller = readFileSync('scripts/windows/uninstall-system-audio-component.ps1', 'utf8');
const apoRoute = readFileSync('crates/voxveil-windows-audio/src/apo_route.rs', 'utf8');
const systemAudioUi = readFileSync('ui/features/home/SystemAudioEndpoints.tsx', 'utf8');
const homeScreen = readFileSync('ui/features/home/HomeScreen.tsx', 'utf8');
const stateHook = readFileSync('ui/app/useVoxveilState.ts', 'utf8');
const systemAudioBackend = readFileSync('tauri/app/system_audio.rs', 'utf8');
const systemAudioLauncher = readFileSync('tauri/app/system_audio_installer.rs', 'utf8');
const releaseGuide = readFileSync('docs/release/windows-apo-production-gate.md', 'utf8');
const validationMatrix = readFileSync('docs/testing/windows-apo-capx-hlk.md', 'utf8');
const windowsSpec = readFileSync('docs/specs/platform/windows.md', 'utf8');

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
    /function\s+Write-InstallStateSnapshot\s*\(\s*\[bool\]\$BindingReady\s*=\s*\$false(?:\s*,[\s\S]*?)?\)/i,
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
  const finalStateRemoval = uninstaller.lastIndexOf('Remove-Item $statePath -Force -ErrorAction SilentlyContinue');
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
  assert.match(hardBlock, /\$state\.pendingRemovedInfName\s*=\s*\$inf/i);
  assert.match(hardBlock, /\$state\.pendingReboot\s*=\s*\$true/i);
  assert.match(hardBlock, /\$state\.pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  assert.match(hardBlock, /Write-JsonStateAtomically\s+-State\s+\$state\s+-Path\s+\$statePath/i);
  assert.match(hardBlock, /exit\s+3010/i);
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
  const descriptorResolve = installer.indexOf('$binding = Resolve-EndpointDescriptor');
  const baseInstall = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  assert.ok(stateLoad >= 0 && descriptorResolve > stateLoad && baseInstall > descriptorResolve);

  const preflight = installer.slice(stateLoad, baseInstall);
  assert.match(preflight, /previousManagedEndpointId/i);
  assert.match(preflight, /previousEndpointId/i);
  assert.match(preflight, /bindingMode/i);
  assert.match(preflight, /selectedEndpointId/i);
  assert.match(preflight, /-ine\s+\$previousManagedEndpointId/i);
  assert.match(preflight, /Uninstall the currently managed Voxveil APO endpoint/i);
});


test('APO endpoint management remains explicit and single-endpoint until telemetry is endpoint-scoped', () => {
  assert.doesNotMatch(systemAudioUi, /onInstallAll|installAll/i);
  assert.doesNotMatch(homeScreen, /installAllSystemAudioEndpoints|onInstallAll/i);
  assert.doesNotMatch(stateHook, /installAllSystemAudioEndpoints/i);

  for (const text of [releaseGuide, validationMatrix, windowsSpec]) {
    assert.match(text, /one .*endpoint at a time|one endpoint at a time|exactly one APO endpoint at a time/i);
    assert.match(text, /bulk/i);
    assert.match(text, /uninstall/i);
  }
});


test('APO lifecycle rejects malformed recorded package identities instead of silently filtering them', () => {
  const installStateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const installPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const installPreflight = installer.slice(installStateLoad, installPnp);
  assert.match(installPreflight, /invalidRecordedInfNames/i);
  assert.match(installPreflight, /Malformed APO\/Extension package identity/i);

  const uninstallStateLoad = uninstaller.indexOf('$state = Get-Content $statePath -Raw | ConvertFrom-Json');
  const uninstallPnp = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const uninstallPreflight = uninstaller.slice(uninstallStateLoad, uninstallPnp);
  assert.match(uninstallPreflight, /invalidRecordedInfNames/i);
  assert.match(uninstallPreflight, /Malformed APO\/Extension package identity/i);
});

test('APO lifecycle rejects unknown persisted binding mode before package mutation', () => {
  const modesPattern = /'capx-extension'\s*,\s*'legacy-runtime-interface'\s*,\s*'legacy-reference'/i;

  const installStateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const installPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const installPreflight = installer.slice(installStateLoad, installPnp);
  assert.match(installPreflight, modesPattern);
  assert.match(installPreflight, /unknown bindingMode/i);

  const uninstallStateLoad = uninstaller.indexOf('$state = Get-Content $statePath -Raw | ConvertFrom-Json');
  const uninstallPnp = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const uninstallPreflight = uninstaller.slice(uninstallStateLoad, uninstallPnp);
  assert.match(uninstallPreflight, modesPattern);
  assert.match(uninstallPreflight, /unknown bindingMode/i);
});


test('APO lifecycle state preflight is singular and uninstaller has no trailing duplicate block', () => {
  assert.equal((installer.match(/\$invalidRecordedInfNames\s*=/g) ?? []).length, 1);
  assert.equal((uninstaller.match(/\$invalidRecordedInfNames\s*=/g) ?? []).length, 1);
  assert.match(
    uninstaller.trimEnd(),
    /Write-Host 'Recorded Voxveil componentized APO packages removed\.'$/,
  );
});


test('TestSign APO certificate ownership is persisted, reused, and removed only after final package cleanup', () => {
  assert.match(installer, /developmentCertificateThumbprint/i);
  assert.match(installer, /previousDevelopmentCertificateThumbprint/i);
  assert.match(installer, /Cert:\\LocalMachine\\My\\\$previousDevelopmentCertificateThumbprint/i);
  assert.match(installer, /New-SelfSignedCertificate/i);
  assert.match(installer, /developmentCertificateThumbprint\s*=\s*\$developmentCertificateThumbprint/i);

  const finalPackageDelete = uninstaller.lastIndexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const certificateCleanup = uninstaller.lastIndexOf('Remove-RecordedDevelopmentCertificate $developmentCertificateThumbprint');
  const stateRemoval = uninstaller.lastIndexOf('Remove-Item $statePath -Force');
  assert.match(uninstaller, /function\s+Remove-RecordedDevelopmentCertificate/i);
  assert.match(uninstaller, /Cert:\\LocalMachine\\My/i);
  assert.match(uninstaller, /Cert:\\LocalMachine\\Root/i);
  assert.match(uninstaller, /Cert:\\LocalMachine\\TrustedPublisher/i);
  assert.ok(certificateCleanup > finalPackageDelete, 'development trust must remain until recorded APO packages are removed');
  assert.ok(stateRemoval > certificateCleanup, 'certificate cleanup failure must preserve install-state ownership for retry');
});

test('APO lifecycle validates recorded development certificate thumbprint before mutation', () => {
  const installStateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const installPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const installPreflight = installer.slice(installStateLoad, installPnp);
  assert.match(installPreflight, /previousDevelopmentCertificateThumbprint/i);
  assert.match(installPreflight, /\^\[0-9A-Fa-f\]\{40\}\\z/i);

  const uninstallStateLoad = uninstaller.indexOf('$state = Get-Content $statePath -Raw | ConvertFrom-Json');
  const uninstallPnp = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const uninstallPreflight = uninstaller.slice(uninstallStateLoad, uninstallPnp);
  assert.match(uninstallPreflight, /developmentCertificateThumbprint/i);
  assert.match(uninstallPreflight, /\^\[0-9A-Fa-f\]\{40\}\\z/i);
});


test('TestSign certificate ownership is checkpointed before trust-store or PnP mutation', () => {
  const certCreate = installer.indexOf('$certificate = New-SelfSignedCertificate');
  const ownershipSnapshot = installer.indexOf('Write-InstallStateSnapshot', certCreate);
  const rootTrust = installer.indexOf('certutil.exe -addstore -f Root', certCreate);
  const firstPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  assert.ok(certCreate >= 0, 'TestSign certificate creation must exist');
  assert.ok(ownershipSnapshot > certCreate, 'new development certificate ownership must be persisted after creation');
  assert.ok(rootTrust > ownershipSnapshot, 'certificate ownership must be recoverable before trust-store mutation can fail');
  assert.ok(firstPnp > rootTrust, 'package mutation must follow scoped certificate ownership persistence');
});


test('APO uninstall retries a required AudioSrv restart after packages are already gone', () => {
  assert.match(uninstaller, /audioServiceRestartRequired/i);

  const zeroOwnership = uninstaller.indexOf('if ($infNames.Count -eq 0)');
  const packageDelete = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  const finalRestart = uninstaller.lastIndexOf('Restart-Service Audiosrv -Force');
  const certificateCleanup = uninstaller.lastIndexOf('Remove-RecordedDevelopmentCertificate $developmentCertificateThumbprint');
  assert.ok(zeroOwnership >= 0 && packageDelete > zeroOwnership && finalRestart > packageDelete);

  const zeroBlock = uninstaller.slice(zeroOwnership, packageDelete);
  assert.match(zeroBlock, /audioServiceRestartRequired/i);
  assert.match(zeroBlock, /Restart-Service Audiosrv -Force/i);
  assert.match(zeroBlock, /audioServiceRestartRequired\s*=\s*\$false/i);

  const deleteTail = uninstaller.slice(packageDelete, finalRestart);
  assert.match(deleteTail, /audioServiceRestartRequired\s*=\s*\(\$infNames\.Count\s*-eq\s*0\)/i);
  assert.match(deleteTail, /Write-JsonStateAtomically\s+-State\s+\$state\s+-Path\s+\$statePath/i);

  const finalTail = uninstaller.slice(finalRestart, certificateCleanup);
  assert.match(finalTail, /audioServiceRestartRequired\s*=\s*\$false/i);
  assert.match(finalTail, /Write-JsonStateAtomically\s+-State\s+\$state\s+-Path\s+\$statePath/i);
});


test('APO installer refuses unfinished AudioSrv cleanup state before PnP mutation', () => {
  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const firstPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const preflight = installer.slice(stateLoad, firstPnp);
  assert.match(preflight, /audioServiceRestartRequired/i);
  assert.match(preflight, /Complete the prior Voxveil APO cleanup/i);
});


test('APO installer refuses persisted binding-mode transitions before package mutation', () => {
  const bindingMode = installer.indexOf("$bindingMode = if (-not $TestSign)");
  const firstPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  assert.ok(bindingMode >= 0 && firstPnp > bindingMode);

  const preflight = installer.slice(bindingMode, firstPnp);
  assert.match(preflight, /previousBindingMode/i);
  assert.match(preflight, /\$bindingMode\s*-ine\s*\$previousBindingMode/i);
  assert.match(preflight, /Uninstall the currently managed Voxveil APO state before changing binding mode/i);
});


test('legacy runtime FX attachment is checkpointed separately from binding mode', () => {
  assert.match(installer, /legacyRuntimeAttached/i);
  assert.match(installer, /legacyRuntimeAttached\s*=\s*\$legacyRuntimeAttached/i);

  const attach = installer.indexOf('& $control attach-effects');
  const markAttached = installer.indexOf('$legacyRuntimeAttached = $true', attach);
  const attachedSnapshot = installer.indexOf('Write-InstallStateSnapshot', markAttached);
  assert.ok(attach >= 0, 'legacy attach mutation must exist');
  assert.ok(markAttached > attach, 'legacy attachment ownership must be marked only after attach succeeds');
  assert.ok(attachedSnapshot > markAttached, 'successful legacy attachment must be checkpointed immediately');

  const detach = uninstaller.indexOf('& $control detach-effects');
  const clearAttached = uninstaller.indexOf('$state.legacyRuntimeAttached = $false', detach);
  const detachSnapshot = uninstaller.indexOf('Write-JsonStateAtomically -State $state -Path $statePath', clearAttached);
  const firstDelete = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force');
  assert.match(uninstaller, /legacyRuntimeAttached/i);
  assert.ok(detach >= 0 && clearAttached > detach, 'legacy detach success must clear attachment ownership');
  assert.ok(detachSnapshot > clearAttached && firstDelete > detachSnapshot, 'cleared attachment state must persist before package deletion');
});

test('legacy runtime uninstall skips detach when install never reached attachment', () => {
  const detachCondition = uninstaller.slice(
    uninstaller.indexOf('if ($state -and'),
    uninstaller.indexOf('& $control detach-effects'),
  );
  assert.match(detachCondition, /legacyRuntimeAttached/i);
});


test('legacy APO repair refuses old state without recorded TestSign certificate ownership', () => {
  const stateLoad = installer.indexOf('$previousState = Get-Content $statePath -Raw | ConvertFrom-Json');
  const firstPnp = installer.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  const preflight = installer.slice(stateLoad, firstPnp);

  assert.match(preflight, /previousBindingMode/i);
  assert.match(preflight, /previousDevelopmentCertificateThumbprint/i);
  assert.match(preflight, /previousInstalledInfNames\.Count\s*-gt\s*0/i);
  assert.match(preflight, /legacy TestSign certificate ownership is unknown/i);
});

test('legacy APO uninstall never guesses an unrecorded development certificate', () => {
  assert.match(uninstaller, /legacy TestSign certificate ownership is unknown/i);
  assert.doesNotMatch(
    uninstaller,
    /Get-ChildItem\s+Cert:\\LocalMachine\\(?:My|Root|TrustedPublisher)[\s\S]*CN=Voxveil Development APO/i,
  );
});


test('APO lifecycle state checkpoints use same-directory atomic replacement', () => {
  for (const [label, text] of [['installer', installer], ['uninstaller', uninstaller]]) {
    assert.match(text, /function\s+Write-JsonStateAtomically/i, `${label} must define atomic state writes`);
    assert.match(text, /Set-Content\s+\$tempPath\s+-Encoding\s+utf8/i);
    assert.match(text, /\[IO\.File\]::Replace\(\$tempPath,\s*\$Path,\s*\$null\)/i);
    assert.match(text, /\[IO\.File\]::Move\(\$tempPath,\s*\$Path\)/i);
    assert.doesNotMatch(text, /ConvertTo-Json\s+-Depth\s+3\s*\|\s*Set-Content\s+\$statePath/i);
  }
});


test('APO uninstall recovers an already-absent recorded package through a conservative reboot checkpoint', () => {
  const loop = uninstaller.indexOf('foreach ($inf in @($infNames))');
  const identity = uninstaller.indexOf('Assert-RecordedApoInfIdentity $inf', loop);
  const deleteDriver = uninstaller.indexOf('pnputil.exe /delete-driver $inf /uninstall /force', loop);
  assert.ok(loop >= 0 && identity > loop && deleteDriver > identity);

  const preDelete = uninstaller.slice(loop, identity);
  assert.match(preDelete, /Test-RecordedApoInfPresent\s+\$inf/i);
  assert.match(preDelete, /if\s*\(\s*-not\s+\$packagePresentBeforeDelete\s*\)/i);
  assert.match(preDelete, /\$state\.installedInfNames\s*=\s*@\(\$infNames\)/i);
  assert.match(preDelete, /\$state\.pendingRemovedInfName\s*=\s*\$inf/i);
  assert.match(preDelete, /\$state\.pendingReboot\s*=\s*\$true/i);
  assert.match(preDelete, /\$state\.pendingRebootBootMarker\s*=\s*\$currentBootMarker/i);
  assert.match(preDelete, /Write-JsonStateAtomically\s+-State\s+\$state\s+-Path\s+\$statePath/i);
  assert.match(preDelete, /exit\s+3010/i);
});


test('elevated APO descriptor handoff is bound to the exact serialized bytes with SHA-256', () => {
  assert.match(systemAudioBackend, /use\s+sha2::\{Digest,\s*Sha256\}/i);
  assert.match(systemAudioBackend, /Sha256::digest\(&json\)/i);
  assert.match(systemAudioBackend, /launch_system_audio_installer\(&script,\s*&descriptor_path,\s*&descriptor_sha256\)/i);

  assert.match(systemAudioLauncher, /descriptor_sha256:\s*&str/i);
  assert.match(systemAudioLauncher, /EndpointDescriptorSha256/i);

  assert.match(installer, /\[ValidatePattern\('\^\[0-9A-Fa-f\]\{64\}\$'\)\]/i);
  assert.match(installer, /\$EndpointDescriptorSha256/i);
  const resolverStart = installer.indexOf('function Resolve-EndpointDescriptor');
  const resolverEnd = installer.indexOf('\nAssert-Administrator\n', resolverStart);
  const resolver = installer.slice(resolverStart, resolverEnd);
  assert.match(resolver, /ReadAllBytes\(\$DescriptorPath\)/i);
  assert.match(resolver, /Security\.Cryptography\.SHA256\]::Create\(\)/i);
  assert.match(resolver, /ComputeHash\(\$descriptorBytes\)/i);
  assert.match(resolver, /endpoint descriptor integrity check failed/i);
  assert.ok(
    resolver.indexOf('endpoint descriptor integrity check failed') <
      resolver.indexOf('ConvertFrom-Json'),
    'descriptor hash must be checked before JSON parsing',
  );
});


test('endpoint descriptor temp file is created exclusively instead of overwriting an existing path', () => {
  assert.match(systemAudioBackend, /OpenOptions::new\(\)/i);
  assert.match(systemAudioBackend, /create_new\(true\)/i);
  assert.match(systemAudioBackend, /ErrorKind::AlreadyExists/i);
  assert.match(systemAudioBackend, /write_all\(json\)/i);
  assert.doesNotMatch(systemAudioBackend, /std::fs::write\(&descriptor_path/i);
});

test('UAC launcher anchors privileged installer script to bytes embedded in the executable', () => {
  assert.match(systemAudioLauncher, /include_bytes!\([^)]*install-system-audio-component\.ps1/i);
  assert.match(systemAudioLauncher, /Sha256::digest/i);
  assert.match(systemAudioLauncher, /std::fs::read\(script\)/i);
  assert.match(systemAudioLauncher, /installer failed integrity verification/i);
  assert.match(systemAudioLauncher, /Get-FileHash[\s\S]{0,120}\$script[\s\S]{0,80}SHA256/i);
  assert.match(systemAudioLauncher, /integrity check failed/i);
  assert.match(systemAudioLauncher, /EncodedCommand/i);
  assert.doesNotMatch(
    systemAudioLauncher,
    /Start-Process[\s\S]{0,500}'-File',\$scriptArg/i,
  );
});


test('production UAC path binds discovery and control helpers to build-time SHA-256 values', () => {
  for (const name of [
    'VOXVEIL_DISCOVERY_SHA256',
    'VOXVEIL_CONTROL_SHA256',
    'VOXVEIL_CONTROL_DLL_SHA256',
  ]) {
    assert.match(systemAudioLauncher, new RegExp(name));
  }
  assert.match(systemAudioLauncher, /option_env!/);
  assert.match(systemAudioLauncher, /verify_trusted_packaged_file/i);
  assert.match(systemAudioLauncher, /DiscoveryHelperSha256/i);
  assert.match(systemAudioLauncher, /ControlHelperSha256/i);
  assert.match(systemAudioLauncher, /ControlDllSha256/i);

  assert.match(installer, /DiscoveryHelperSha256/i);
  assert.match(installer, /ControlHelperSha256/i);
  assert.match(installer, /ControlDllSha256/i);
  assert.match(installer, /Assert-TrustedPackagedFile/i);
  assert.match(installer, /discover-system-audio-endpoints\.ps1/i);
  assert.match(installer, /voxveil-control\.exe/i);
  assert.match(installer, /VoxveilControl\.dll/i);
});


test('production elevation resolves Windows PowerShell from the OS system directory', () => {
  assert.match(systemAudioLauncher, /voxveil_windows_audio::windows_system_directory\(\)/i);
  assert.doesNotMatch(systemAudioLauncher, /\bunsafe\b/i);
  assert.match(systemAudioLauncher, /WindowsPowerShell[\\/]v1\.0[\\/]powershell\.exe/i);
  assert.doesNotMatch(systemAudioLauncher, /SystemRoot/i);
  assert.doesNotMatch(systemAudioLauncher, /Command::new\("powershell\.exe"\)/i);
  assert.doesNotMatch(systemAudioLauncher, /Start-Process\s+-FilePath\s+'powershell\.exe'/i);

  const resolverStart = installer.indexOf('function Resolve-EndpointDescriptor');
  const resolverEnd = installer.indexOf('\nAssert-Administrator\n', resolverStart);
  const resolver = installer.slice(resolverStart, resolverEnd);
  assert.match(resolver, /\[Environment\]::SystemDirectory/i);
  assert.match(resolver, /WindowsPowerShell\\v1\.0\\powershell\.exe/i);
  assert.doesNotMatch(resolver, /\$env:(?:SystemRoot|windir)/i);
  assert.doesNotMatch(resolver, /&\s+powershell\.exe/i);
});


test('production control helper integrity is rechecked immediately before readiness execution', () => {
  const status = installer.indexOf('$status = & $control status');
  assert.ok(status >= 0);
  const preStatus = installer.slice(Math.max(0, status - 900), status);
  assert.match(preStatus, /if\s*\(\s*-not\s+\$TestSign\s*\)/i);
  assert.match(preStatus, /Assert-TrustedPackagedFile\s+\$control\s+\$ControlHelperSha256/i);
  assert.match(preStatus, /Assert-TrustedPackagedFile\s+\$controlDll\s+\$ControlDllSha256/i);
});


test('production APO install locks trusted files from verification through privileged execution', () => {
  assert.match(installer, /function\s+Open-TrustedReadLock/i);
  assert.match(installer, /IO\.FileMode\]::Open/i);
  assert.match(installer, /IO\.FileAccess\]::Read/i);
  assert.match(installer, /IO\.FileShare\]::Read/i);
  assert.match(installer, /\$productionPackageLocks/i);
  assert.match(installer, /\$controlLock/i);
  assert.match(installer, /\$controlDllLock/i);
  assert.match(installer, /\.Dispose\(\)/i);
});

test('production APO PnP installs directly from the locked staged package instead of user temp copies', () => {
  assert.match(installer, /\$apoInstallInf\s*=\s*if\s*\(\$TestSign\)/i);
  assert.match(installer, /\$extensionInstallInf\s*=\s*if\s*\(\$TestSign\)/i);
  assert.match(installer, /pnputil\.exe\s+\/add-driver\s+\$apoInstallInf\s+\/install/i);
  assert.match(installer, /pnputil\.exe\s+\/add-driver\s+\$extensionInstallInf\s+\/install/i);
  assert.doesNotMatch(installer, /Copy-Item\s+\$prebuiltExtension\s+\$extensionInf/i);
  assert.doesNotMatch(installer, /Copy-Item\s+\$apoCat,\s*\$extensionCat\s+-Destination\s+\$work/i);
});

test('elevated launcher keeps the installer script read-locked between hash verification and execution', () => {
  assert.match(systemAudioLauncher, /IO\.File\]::Open/i);
  assert.match(systemAudioLauncher, /IO\.FileShare\]::Read/i);
  assert.match(systemAudioLauncher, /ComputeHash\([^)]*scriptLock/i);
  assert.match(systemAudioLauncher, /scriptLock\.Dispose\(\)/i);
});
