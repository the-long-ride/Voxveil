import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const root = process.cwd();
const apoRoot = path.join(root, 'native', 'windows', 'apo');
const clsid = '{7E268E67-2F3C-4F0A-A09C-8B7D27B43F51}';

function read(name) {
  return fs.readFileSync(path.join(apoRoot, name), 'utf8');
}

function readRepo(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

test('APO package contains native runtime-registration sources', () => {
  for (const file of [
    'target_discovery.cpp',
    'VoxveilApoTarget.vcxproj',
    'install.ps1',
    'uninstall.ps1',
    'README.txt',
    'com_smoke.cpp',
    'VoxveilApoCheck.vcxproj',
  ]) {
    assert.equal(fs.existsSync(path.join(apoRoot, file)), true, `${file} must exist`);
  }
});

test('production INF sources remain available for a future signed release', () => {
  const inf = read('VoxveilApo.inf');
  const extension = read('extension.ps1');
  assert.match(inf, /Class\s*=\s*AudioProcessingObject/i);
  assert.match(inf, /5989fce8-9cd0-467d-8a6a-5419e31529d4/i);
  assert.match(inf, /SWC\\VEN_VOXV&CID_APO/i);
  assert.match(extension, /AddComponent\s*=\s*VoxveilApo/i);
  assert.match(extension, /PKEY_CompositeFX_EndpointEffectClsid/i);
});

test('runtime target helper uses SetupAPI device-interface storage for FX registration', () => {
  const source = read('target_discovery.cpp');
  const project = read('VoxveilApoTarget.vcxproj');
  assert.match(source, /SetupDiGetClassDevsW/);
  assert.match(source, /SetupDiEnumDeviceInterfaces/);
  assert.match(source, /SetupDiGetDeviceInterfaceDetailW/);
  assert.match(source, /SetupDiCreateDeviceInterfaceRegKeyW?/);
  assert.match(source, /RegCreateKeyExW/);
  assert.match(source, /FX\\\\0|L"FX\\\\0"/);
  assert.match(source, /D04E05A6-594B-4FB6-A80D-01AF5EED7D1D/i);
  assert.match(source, /CompositeFX_EndpointEffectClsid|\},15/);
  assert.match(source, /D3993A3F-99C2-4402-B5EC-A92A0367664B/i);
  assert.match(source, /C18E2F7E-933D-4965-B7D1-1EEF228D2AF3/i);
  assert.match(source, /--install-fx/);
  assert.match(source, /--remove-fx/);
  assert.match(project, /setupapi\.lib/i);
  assert.match(project, /advapi32\.lib/i);
  assert.match(project, /<RuntimeLibrary>MultiThreaded<\/RuntimeLibrary>/i);
});

test('development installer uses runtime interface registration and avoids driver/Test Mode hacks', () => {
  const install = read('install.ps1');
  const uninstall = read('uninstall.ps1');
  assert.match(install, /VoxveilApoTarget\.exe/);
  assert.match(install, /--install-fx/);
  assert.match(uninstall, /--remove-fx/);
  for (const text of [install, uninstall]) {
    assert.doesNotMatch(text, /MMDevices\\Audio\\Render/i);
    assert.doesNotMatch(text, /\/add-driver/i);
    assert.doesNotMatch(text, /\/delete-driver/i);
    assert.doesNotMatch(text, /bcdedit/i);
    assert.doesNotMatch(text, /testsigning/i);
    assert.doesNotMatch(text, /takeown/i);
  }
});

test('single Windows binary embeds only the runtime installation payload', () => {
  const systemAudio = readRepo('tauri/app/system_audio.rs');
  const build = readRepo('tauri/build.rs');

  assert.match(systemAudio, /include_bytes!\([^)]*VoxveilApo\.dll/i);
  assert.match(systemAudio, /include_bytes!\([^)]*VoxveilApoCheck\.exe/i);
  assert.match(systemAudio, /include_bytes!\([^)]*VoxveilApoTarget\.exe/i);
  assert.match(systemAudio, /include_str!\([^)]*install\.ps1/i);
  assert.match(systemAudio, /include_str!\([^)]*uninstall\.ps1/i);
  assert.doesNotMatch(systemAudio, /include_str!\([^)]*VoxveilApo\.inf/i);
  assert.doesNotMatch(systemAudio, /include_str!\([^)]*extension\.ps1/i);
  assert.doesNotMatch(systemAudio, /installer_path_for_exe/);

  assert.match(build, /VoxveilApo\.dll/);
  assert.match(build, /VoxveilApoCheck\.exe/);
  assert.match(build, /VoxveilApoTarget\.exe/);
});

test('installer validates the DLL through the native COM checker', () => {
  const install = read('install.ps1');
  const checker = read('com_smoke.cpp');
  assert.match(install, /Test-ApoComServer/);
  assert.match(install, /VoxveilApoCheck\.exe/);
  assert.match(checker, /DllGetClassObject/);
  assert.match(checker, /IClassFactory::CreateInstance/);
  assert.match(checker, /IAudioProcessingObject/);
  assert.equal(clsid.length > 0, true);
});

test('development protected-audio change is disclosed and restored without enabling test mode', () => {
  const install = read('install.ps1');
  const uninstall = read('uninstall.ps1');
  const readme = read('README.txt');
  for (const text of [install, uninstall, readme]) {
    assert.match(text, /DisableProtectedAudioDG/i);
    assert.doesNotMatch(text, /bcdedit/i);
    assert.doesNotMatch(text, /testsigning/i);
  }
  assert.match(install, /protected-audio-backup\.json/i);
  assert.match(uninstall, /protected-audio-backup\.json/i);
});

test('APO and helpers are statically linked for portable development use', () => {
  const project = read('VoxveilApo.vcxproj');
  const checker = read('VoxveilApoCheck.vcxproj');
  const target = read('VoxveilApoTarget.vcxproj');
  assert.match(project, /<EmbedManifest>false<\/EmbedManifest>/i);
  assert.match(project, /<RuntimeLibrary[^>]*>MultiThreaded<\/RuntimeLibrary>/i);
  assert.match(checker, /<RuntimeLibrary>MultiThreaded<\/RuntimeLibrary>/i);
  assert.match(target, /<RuntimeLibrary>MultiThreaded<\/RuntimeLibrary>/i);
});
