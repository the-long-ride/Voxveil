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
  const manifest = text.search(/verification\.json/i);
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
