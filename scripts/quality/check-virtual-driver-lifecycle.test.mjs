import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

const installer = () => read('scripts/windows/install-staged-virtual-driver.ps1');
const uninstaller = () => read('scripts/windows/uninstall-staged-virtual-driver.ps1');

test('virtual driver installer requires the staged verification manifest and rehashes all signed files', () => {
  const text = installer();
  assert.match(text, /verification\.json/i);
  assert.match(text, /Get-FileHash/i);
  assert.match(text, /VoxveilVirtualAudio\.inf/i);
  assert.match(text, /VoxveilVirtualAudio\.cat/i);
  assert.match(text, /VoxveilVirtualAudio\.sys/i);
  assert.match(text, /infSha256/);
  assert.match(text, /catalogSha256/);
  assert.match(text, /driverSha256/);
  assert.match(text, /Root\\VoxveilVirtualAudio/i);
  assert.match(text, /79E4E58C-9714-44E8-ACA1-426F24B7A1E9/i);
});

test('signed virtual driver stager rehashes copied files before writing verification manifest', () => {
  const text = read('scripts/windows/stage-signed-virtual-driver.ps1');
  const copy = text.search(/Copy-Item/i);
  const stagedHash = text.indexOf('Assert-StagedHash -Path $stagedInf', copy);
  const manifest = text.indexOf("Set-Content (Join-Path $destination 'verification.json')", stagedHash);
  assert.ok(copy >= 0, 'stager must copy verified package members');
  assert.ok(stagedHash > copy, 'stager must validate destination hashes after copying');
  assert.ok(manifest > stagedHash, 'verification manifest must be written only after destination hashes pass');
  assert.match(text, /function\s+Assert-StagedHash[\s\S]*?Get-FileHash/i);
  assert.match(text, /Assert-StagedHash -Path \$stagedCat -Expected \$verification\.catalogSha256/i);
  assert.match(text, /Assert-StagedHash -Path \$stagedSys -Expected \$verification\.driverSha256/i);
  assert.match(text, /staged.*hash/i);
});

test('virtual driver installer never changes TESTSIGNING or imports certificates', () => {
  const text = installer();
  assert.doesNotMatch(text, /bcdedit[^\r\n]*(?:set|deletevalue)/i);
  assert.doesNotMatch(text, /New-SelfSignedCertificate|Import-Certificate|certutil[^\r\n]+-addstore/i);
  assert.match(text, /pnputil(?:\.exe)?\s+\/add-driver/i);
});

test('virtual driver installer rejects unsupported Windows before creating the root devnode', () => {
  const text = installer();
  assert.match(text, /CurrentBuildNumber/i);
  assert.match(text, /22621/);
  assert.match(text, /Assert-SupportedWindowsBuild/i);
  assert.match(text, /Assert-Administrator\s*\r?\nAssert-SupportedWindowsBuild/i);
  const preflight = text.search(/Assert-Administrator\s*\r?\nAssert-SupportedWindowsBuild/i);
  const ensure = text.search(/&\s*\$deviceHelper\s+ensure/i);
  assert.ok(preflight >= 0 && ensure > preflight, 'OS floor must be checked before ensuring the devnode');
});

test('Voxveil owns a minimal SetupAPI helper for exact root devnode lifecycle', () => {
  const project = read('native/windows/driver/VoxveilVirtualAudioDevice.vcxproj');
  const source = read('native/windows/driver/VoxveilVirtualAudioDevice.cpp');
  assert.match(project, /<ConfigurationType>Application<\/ConfigurationType>/i);
  assert.match(project, /Setupapi\.lib/i);
  assert.match(project, /Release\|x64/i);
  assert.match(project, /Release\|ARM64/i);
  assert.match(source, /Root\\\\VoxveilVirtualAudio/i);
  assert.match(source, /SetupDiGetINFClassW/i);
  assert.match(source, /SetupDiCreateDeviceInfoW/i);
  assert.match(source, /SPDRP_HARDWAREID/i);
  assert.match(source, /DIF_REGISTERDEVICE/i);
  assert.match(source, /SetupDiOpenDeviceInfoW/i);
  assert.match(source, /DIF_REMOVE/i);
  assert.doesNotMatch(source, /DevCon|UpdateDriverForPlugAndPlayDevices/i);
});

test('SetupAPI helper imports the Configuration Manager class-name limit it uses', () => {
  const source = read('native/windows/driver/VoxveilVirtualAudioDevice.cpp');
  assert.match(source, /#include\s*<cfgmgr32\.h>/i);
  assert.match(source, /wchar_t\s+className\s*\[\s*MAX_CLASS_NAME_LEN\s*\]/i);
  assert.match(source, /SetupDiGetINFClassW\s*\([\s\S]*?className[\s\S]*?std::size\(className\)[\s\S]*?nullptr\s*\)/i);
});

test('virtual driver install creates the exact devnode before PnPUtil and records its instance ID', () => {
  const text = installer();
  assert.match(text, /voxveil-virtual-device\.exe/i);
  assert.match(text, /\bensure\b/i);
  assert.match(text, /deviceInstanceId/i);
  assert.match(text, /created/i);
  assert.match(text, /pnputil(?:\.exe)?\s+\/add-driver/i);
  assert.ok(
    text.search(/voxveil-virtual-device\.exe/i) < text.search(/pnputil(?:\.exe)?\s+\/add-driver/i),
    'root devnode must exist before PnPUtil attempts driver installation',
  );
  assert.match(text, /if\s*\(\s*\$deviceCreated[\s\S]*?\bremove\b/i);
});

test('failed virtual driver install rolls back only the driver-store package added by this attempt', () => {
  const text = installer();
  assert.match(text, /beforePublishedInfNames/);
  assert.match(text, /afterPublishedInfNames/);
  assert.match(text, /newPublishedInf/);
  assert.match(
    text,
    /Where-Object\s*\{\s*\$beforePublishedInfNames\s*-inotcontains\s*\$_\s*\}/s,
    'rollback ownership must come from the before/after published-INF delta',
  );

  const catchStart = text.indexOf('catch {');
  assert.ok(catchStart >= 0, 'installer must have a failure rollback block');
  const rollback = text.slice(catchStart);
  const removeDevnode = rollback.search(/&\s*\$deviceHelper\s+remove\s+\$deviceInstanceId/i);
  const deletePackage = rollback.search(/pnputil(?:\.exe)?\s+\/delete-driver\s+\$newPublishedInf/i);
  assert.ok(removeDevnode >= 0, 'rollback must remove a newly created devnode first');
  assert.ok(deletePackage > removeDevnode, 'rollback must delete only the newly added package after devnode removal');
});

test('PnPUtil failure refreshes Driver Store ownership before throwing so staged packages remain rollbackable', () => {
  const text = installer();
  const addDriver = text.indexOf('pnputil.exe /add-driver $inf /install');
  assert.ok(addDriver >= 0, 'installer must invoke PnPUtil for the staged INF');
  const tail = text.slice(addDriver);
  const captureExit = tail.search(/\$pnputilExitCode\s*=\s*\$LASTEXITCODE/i);
  const refreshInventory = tail.search(/\$afterPublishedInfNames\s*=\s*@\(Get-VoxveilPublishedInfNames\)/i);
  const failureCheck = tail.search(/if\s*\(\s*\$pnputilExitCode\s*-ne\s*0\s*\)/i);
  assert.ok(captureExit >= 0, 'PnPUtil exit code must be captured before any ownership refresh');
  assert.ok(refreshInventory > captureExit, 'Driver Store inventory must refresh after PnPUtil returns');
  assert.ok(failureCheck > refreshInventory, 'PnPUtil failure must be thrown only after rollback ownership is refreshed');
});

test('virtual driver reboot-required success records exact cleanup state before exiting', () => {
  const text = installer();
  const addDriver = text.indexOf('pnputil.exe /add-driver $inf /install');
  const catchStart = text.indexOf('catch {', addDriver);
  const block = text.slice(addDriver, catchStart);
  assert.ok(addDriver >= 0 && catchStart > addDriver);

  const hardFailureGate = block.search(/if\s*\(\s*\$pnputilExitCode\s*-ne\s*0\s*\)/i);
  const rebootExemption = block.search(/if\s*\(\s*\$pnputilExitCode\s*-ne\s*3010\s*\)/i);
  const rebootAggregate = block.search(/\$lifecycleRebootRequired\s*=\s*\$helperRebootRequired\s*-or\s*\$pnputilExitCode\s*-eq\s*3010/i);
  const stateWrite = block.search(/Write-VirtualDriverInstallState\s+-PublishedInf\s+\$publishedInf\s+-PendingReboot\s+\$true/i);
  const rebootExit = block.search(/exit\s+3010/i);

  assert.ok(hardFailureGate >= 0, 'nonzero PnPUtil results must be checked after Driver Store ownership refresh');
  assert.ok(rebootExemption > hardFailureGate, '3010 must be exempted from the hard package failure path');
  assert.ok(rebootAggregate > rebootExemption, 'SetupAPI and PnPUtil restart signals must be aggregated after package result validation');
  assert.match(block.slice(rebootAggregate), /\$newPublishedInf/i);
  assert.match(block.slice(rebootAggregate), /\$installedDrivers\.Count\s*-eq\s*1/i);
  assert.ok(stateWrite > rebootAggregate, 'exact package/devnode ownership must be persisted after the combined restart decision');
  assert.ok(rebootExit > stateWrite, 'reboot-required exit must follow the persisted cleanup state');
});

test('virtual driver rollback ownership reads only the exact package from Driver Store inventory', () => {
  const text = installer();
  const helperMatch = text.match(/function\s+Get-VoxveilPublishedInfNames\s*\{([\s\S]*?)\n\}/i);
  assert.ok(helperMatch, 'installer must define Get-VoxveilPublishedInfNames');
  const helper = helperMatch[1];

  assert.match(helper, /Get-WindowsDriver\s+-Online/i);
  assert.match(helper, /ProviderName\s+-ieq\s*'Voxveil'/i);
  assert.match(helper, /OriginalFileName/i);
  assert.match(helper, /GetFileName\s*\(/i);
  assert.match(helper, /VoxveilVirtualAudio\.inf/i);
  assert.match(helper, /\.Driver\b/);
  assert.doesNotMatch(helper, /Win32_PnPSignedDriver/i);
});

test('virtual driver lifecycle records and deletes only its exact published INF and devnode', () => {
  const install = installer();
  const uninstall = uninstaller();
  assert.match(install, /virtual-driver-install-state\.json/i);
  assert.match(install, /DeviceName\s+-eq\s*'Voxveil Virtual Audio'/i);
  assert.match(install, /publishedInf/i);
  assert.match(install, /deviceInstanceId/i);
  assert.match(uninstall, /virtual-driver-install-state\.json/i);
  assert.match(uninstall, /publishedInf/i);
  assert.match(uninstall, /deviceInstanceId/i);
  assert.match(uninstall, /Get-CimInstance\s+Win32_PnPSignedDriver/i);
  assert.match(uninstall, /InfName\s+-ieq\s+\$PublishedInf/i);
  assert.match(uninstall, /DriverProviderName\s+-ine\s*'Voxveil'/i);
  assert.match(uninstall, /DeviceName\s+-ine\s*'Voxveil Virtual Audio'/i);
  assert.match(uninstall, /voxveil-virtual-device\.exe/i);
  assert.match(uninstall, /\bremove\b/i);
  assert.match(uninstall, /pnputil(?:\.exe)?\s+\/delete-driver/i);
  assert.ok(
    uninstall.search(/voxveil-virtual-device\.exe/i) < uninstall.search(/pnputil(?:\.exe)?\s+\/delete-driver/i),
    'the exact devnode must be removed before its driver-store package is deleted',
  );
  assert.doesNotMatch(uninstall, /Get-CimInstance[\s\S]*?Where-Object\s*\{[^}]*DriverProviderName\s+-eq\s*'Voxveil'[^}]*\}[\s\S]*?\/delete-driver/i);
});

test('Windows desktop package builds and includes the scoped virtual-driver lifecycle helper', () => {
  const text = read('scripts/windows/build-windows.ps1');
  assert.match(text, /VoxveilVirtualAudioDevice\.vcxproj/i);
  assert.match(text, /voxveil-virtual-device\.exe/i);
  assert.match(text, /install-staged-virtual-driver\.ps1/i);
  assert.match(text, /uninstall-staged-virtual-driver\.ps1/i);
});

test('native virtual-audio build outputs are ignored', () => {
  const text = read('.gitignore');
  assert.match(text, /native\/windows\/driver\/build\//i);
  assert.match(text, /native\/windows\/driver\/bin\//i);
  assert.match(text, /native\/windows\/driver\/obj\//i);
  assert.match(text, /native\/windows\/driver\/out\//i);
});


test('virtual-driver lifecycle state checkpoints use same-directory atomic replacement', () => {
  for (const [label, text] of [['installer', installer()], ['uninstaller', uninstaller()]]) {
    assert.match(text, /function\s+Write-JsonStateAtomically/i, `${label} must define atomic state writes`);
    assert.match(text, /Set-Content\s+\$tempPath\s+-Encoding\s+utf8/i);
    assert.match(text, /\[IO\.File\]::Replace\(\$tempPath,\s*\$Path,\s*\$null\)/i);
    assert.match(text, /\[IO\.File\]::Move\(\$tempPath,\s*\$Path\)/i);
    assert.doesNotMatch(text, /ConvertTo-Json\s+-Depth\s+3\s*\|\s*Set-Content\s+\$statePath/i);
  }
});


test('retail virtual-driver install revalidates retained qualification evidence', () => {
  const text = installer();
  assert.match(text, /releaseEvidenceSha256/i);
  assert.match(text, /release-evidence\.json/i);
  assert.match(text, /signingPath/i);
  assert.match(text, /whcp-hlk/i);
  assert.match(text, /microsoft-approved-retail/i);
  assert.match(text, /Assert-StagedFileHash\s+\$releaseEvidencePath\s+\$expectedEvidenceSha256\s+'release-evidence\.json'/i);
  assert.match(text, /Retail release evidence changed after staging/i);
});


test('virtual-driver install matches current Microsoft catalog signer to verification manifest', () => {
  const text = installer();
  assert.match(text, /catalogSigner/i);
  assert.match(text, /SignerCertificate\.Subject/i);
  assert.match(text, /catalog signer does not match verification\.json/i);
});


test('virtual-driver install binds current catalog certificate thumbprint to verification manifest', () => {
  const text = installer();
  assert.match(text, /catalogThumbprint/i);
  assert.match(text, /SignerCertificate\.Thumbprint/i);
  assert.match(text, /catalog thumbprint does not match verification\.json/i);
});


test('virtual-driver staging and lifecycle hash-bind the packaged SetupAPI helper', () => {
  const build = read('scripts/windows/build-windows.ps1');
  const stager = read('scripts/windows/stage-signed-virtual-driver.ps1');

  assert.match(build, /\$virtualDeviceHelperSha256\s*=\s*Get-Sha256Hex\s+\$virtualDeviceHelper/i);
  assert.match(build, /-DeviceHelperPath\s+\(Join-Path\s+\$systemAudio\s+'voxveil-virtual-device\.exe'\)/i);
  assert.match(stager, /\[string\]\$DeviceHelperPath/i);
  assert.match(stager, /deviceHelperSha256\s*=\s*\$deviceHelperSha256/i);

  for (const [label, text] of [
    ['installer', installer()],
    ['uninstaller', uninstaller()],
  ]) {
    const helperExecution = text.search(/&\s*\$deviceHelper\s+(?:ensure|query|remove)/i);
    const helperLock = text.search(/\$deviceHelperLock\s*=\s*Open-TrustedVerifiedReadLock\s+\$deviceHelper\s+\$expectedDeviceHelperSha256\s+'voxveil-virtual-device\.exe'/i);
    assert.ok(helperLock >= 0, `${label} must open a verified read lock on the SetupAPI helper`);
    assert.ok(helperExecution > helperLock, `${label} must lock and verify the helper before first execution`);
    assert.match(text, /\[IO\.File\]::Open\([\s\S]*?\[IO\.FileShare\]::Read/i);
    assert.match(text, /ComputeHash\(\$stream\)/i);
    assert.match(text, /finally\s*\{[\s\S]*?\$deviceHelperLock\.Dispose\(\)/i);
    assert.match(text, /deviceHelperSha256/i);
  }
});


test('virtual-driver lifecycle pins PnPUtil to the OS system directory', () => {
  for (const [label, text] of [
    ['installer', installer()],
    ['uninstaller', uninstaller()],
  ]) {
    const firstPnp = text.indexOf('pnputil.exe /');
    assert.ok(firstPnp >= 0, `${label} must invoke PnPUtil`);
    const preflight = text.slice(0, firstPnp);
    assert.match(preflight, /\[Environment\]::SystemDirectory/i);
    assert.match(
      preflight,
      /Set-Alias\s+-Name\s+'pnputil\.exe'\s+-Value\s+\$pnputilPath\s+-Scope\s+Script\s+-Option\s+ReadOnly/i,
    );
  }
});


test('virtual-driver lifecycle pins PowerShell module discovery to the OS module directory', () => {
  for (const [label, text] of [
    ['installer', installer()],
    ['uninstaller', uninstaller()],
  ]) {
    assert.match(text, /\[Environment\]::SystemDirectory/i, `${label} must resolve the OS system directory`);
    assert.match(text, /WindowsPowerShell\\v1\.0\\Modules/i);
    assert.match(text, /\$env:PSModulePath\s*=\s*\$trustedModulePath/i);
  }
});


test('virtual-driver uninstall derives the INF directory from the OS system directory', () => {
  const text = uninstaller();
  assert.match(text, /\$trustedWindowsDirectory\s*=\s*Split-Path\s+-Parent\s+\$trustedSystemDirectoryForModules/i);
  assert.match(text, /Join-Path\s+\$trustedWindowsDirectory\s+"INF\\\$PublishedInf"/i);
  assert.doesNotMatch(text, /\$env:(?:windir|SystemRoot)/i);
});
