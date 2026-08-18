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

test('APO development package has install, uninstall, and native checker sources', () => {
  for (const file of [
    'install.ps1',
    'uninstall.ps1',
    'README.txt',
    'com_smoke.cpp',
    'VoxveilApoCheck.vcxproj',
  ]) {
    assert.equal(fs.existsSync(path.join(apoRoot, file)), true, `${file} must exist`);
  }
});

test('installer registers the same CLSID as the native component and checker', () => {
  const native = read('voxveil_apo.cpp').toUpperCase();
  const checker = read('com_smoke.cpp').toLowerCase();
  const install = read('install.ps1').toUpperCase();
  const uninstall = read('uninstall.ps1').toUpperCase();
  assert.match(install, /7E268E67-2F3C-4F0A-A09C-8B7D27B43F51/);
  assert.match(uninstall, /7E268E67-2F3C-4F0A-A09C-8B7D27B43F51/);
  for (const part of ['7e268e67', '0x2f3c', '0x4f0a']) {
    assert.match(native.toLowerCase(), new RegExp(part));
    assert.match(checker, new RegExp(part));
  }
  assert.equal(clsid.length > 0, true);
});

test('installer validates the DLL through the native COM checker', () => {
  const install = read('install.ps1');
  const checker = read('com_smoke.cpp');
  assert.match(install, /Test-ApoComServer/);
  assert.match(install, /VoxveilApoCheck\.exe/);
  assert.match(checker, /DllGetClassObject/);
  assert.match(checker, /IClassFactory::CreateInstance/);
  assert.match(checker, /IAudioProcessingObject/);
});

test('installer preserves existing endpoint effects and uses composite EFX', () => {
  const install = read('install.ps1');
  const uninstall = read('uninstall.ps1');
  assert.match(install, /D04E05A6-594B-4FB6-A80D-01AF5EED7D1D\},15/i);
  assert.match(install, /endpoint-backup\.json/i);
  assert.match(uninstall, /endpoint-backup\.json/i);
  assert.match(uninstall, /Restore-Endpoint/i);
});

test('installer preserves and enables the endpoint system-effects setting', () => {
  const install = read('install.ps1');
  const uninstall = read('uninstall.ps1');
  for (const text of [install, uninstall]) {
    assert.match(text, /1DA5D803-D492-4EDD-8C23-E0C0FFEE7F0E\},5/i);
  }
  assert.match(install, /SysFxExists/);
  assert.match(install, /SysFxValue/);
  assert.match(install, /PropertyType DWord -Value 0/);
  assert.match(uninstall, /backup\.SysFxExists/);
  assert.match(uninstall, /backup\.SysFxValue/);
});

test('development protected-audio change is disclosed and restored', () => {
  const install = read('install.ps1');
  const uninstall = read('uninstall.ps1');
  const readme = read('README.txt');
  for (const text of [install, uninstall, readme]) {
    assert.match(text, /DisableProtectedAudioDG/i);
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
