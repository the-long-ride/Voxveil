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

test('APO package contains componentized deployment sources', () => {
  for (const file of [
    'VoxveilApo.inf',
    'targets.ps1',
    'extension.ps1',
    'install.ps1',
    'uninstall.ps1',
    'README.txt',
    'com_smoke.cpp',
    'VoxveilApoCheck.vcxproj',
  ]) {
    assert.equal(fs.existsSync(path.join(apoRoot, file)), true, `${file} must exist`);
  }
});

test('APO INF follows the Windows 11 componentized APO model', () => {
  const inf = read('VoxveilApo.inf');
  assert.match(inf, /Class\s*=\s*AudioProcessingObject/i);
  assert.match(inf, /5989fce8-9cd0-467d-8a6a-5419e31529d4/i);
  assert.match(inf, /SWC\\VEN_VOXV&CID_APO/i);
  assert.match(inf, /HKR,Classes\\CLSID/i);
  assert.match(inf, /HKR,AudioEngine\\AudioProcessingObjects/i);
  assert.match(inf, /VoxveilApo\.dll/i);
  assert.match(inf, /7E268E67-2F3C-4F0A-A09C-8B7D27B43F51/i);
});

test('extension generator uses AddComponent and endpoint EFX properties', () => {
  const extension = read('extension.ps1');
  assert.match(extension, /AddComponent\s*=\s*VoxveilApo/i);
  assert.match(extension, /VEN_VOXV&CID_APO/i);
  assert.match(extension, /PKEY_CompositeFX_EndpointEffectClsid/i);
  assert.match(extension, /D04E05A6-594B-4FB6-A80D-01AF5EED7D1D\},15/i);
  assert.match(extension, /PKEY_EFX_ProcessingModes_Supported_For_Streaming/i);
  assert.match(extension, /C18E2F7E-933D-4965-B7D1-1EEF228D2AF3/i);
  assert.match(extension, /AddInterface/i);
});

test('installer uses PnP packages and never mutates protected MMDevices FxProperties', () => {
  const install = read('install.ps1');
  const uninstall = read('uninstall.ps1');
  assert.match(install, /pnputil(?:\.exe)?/i);
  assert.match(install, /\/add-driver/i);
  assert.match(install, /VoxveilApo\.inf/i);
  assert.match(install, /VoxveilAudioExtension\.inf/i);
  assert.doesNotMatch(install, /MMDevices\\Audio\\Render/i);
  assert.doesNotMatch(install, /New-Item\s+-Path\s+\$fx/i);
  assert.doesNotMatch(uninstall, /MMDevices\\Audio\\Render/i);
  assert.doesNotMatch(uninstall, /Restore-Endpoint/i);
  assert.match(uninstall, /\/delete-driver/i);
});

test('single Windows binary embeds the complete INF deployment payload', () => {
  const systemAudio = readRepo('tauri/app/system_audio.rs');
  const build = readRepo('tauri/build.rs');

  assert.match(systemAudio, /include_bytes!\([^)]*VoxveilApo\.dll/i);
  assert.match(systemAudio, /include_bytes!\([^)]*VoxveilApoCheck\.exe/i);
  for (const name of ['VoxveilApo.inf', 'targets.ps1', 'extension.ps1', 'install.ps1', 'uninstall.ps1']) {
    assert.match(systemAudio, new RegExp(`include_str!\\([^)]*${name.replace('.', '\\.')} ` .trim(), 'i'));
  }
  assert.doesNotMatch(systemAudio, /installer_path_for_exe/);

  assert.match(build, /VoxveilApo\.dll/);
  assert.match(build, /VoxveilApoCheck\.exe/);
  assert.match(build, /VoxveilApo\.inf/);
  assert.match(build, /targets\.ps1/);
  assert.match(build, /extension\.ps1/);
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

test('APO and checker are statically linked for portable development use', () => {
  const project = read('VoxveilApo.vcxproj');
  const checker = read('VoxveilApoCheck.vcxproj');
  assert.match(project, /<EmbedManifest>false<\/EmbedManifest>/i);
  assert.match(project, /<RuntimeLibrary[^>]*>MultiThreaded<\/RuntimeLibrary>/i);
  assert.match(checker, /<RuntimeLibrary>MultiThreaded<\/RuntimeLibrary>/i);
});
