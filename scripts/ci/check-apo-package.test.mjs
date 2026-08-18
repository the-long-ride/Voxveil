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

test('APO development package has install and uninstall entry points', () => {
  for (const file of ['install.ps1', 'uninstall.ps1', 'README.txt']) {
    assert.equal(fs.existsSync(path.join(apoRoot, file)), true, `${file} must exist`);
  }
});

test('installer registers the same CLSID as the native component', () => {
  const native = read('voxveil_apo.cpp').toUpperCase();
  const install = read('install.ps1').toUpperCase();
  const uninstall = read('uninstall.ps1').toUpperCase();
  assert.match(install, /7E268E67-2F3C-4F0A-A09C-8B7D27B43F51/);
  assert.match(uninstall, /7E268E67-2F3C-4F0A-A09C-8B7D27B43F51/);
  for (const part of ['7e268e67', '0x2f3c', '0x4f0a']) {
    assert.match(native.toLowerCase(), new RegExp(part));
  }
  assert.equal(clsid.length > 0, true);
});

test('installer preserves existing endpoint effects and uses composite EFX', () => {
  const install = read('install.ps1');
  const uninstall = read('uninstall.ps1');
  assert.match(install, /D04E05A6-594B-4FB6-A80D-01AF5EED7D1D\},15/i);
  assert.match(install, /endpoint-backup\.json/i);
  assert.match(uninstall, /endpoint-backup\.json/i);
  assert.match(uninstall, /Restore-Endpoint/i);
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

test('APO project disables embedded manifest for protected audio compatibility', () => {
  const project = read('VoxveilApo.vcxproj');
  assert.match(project, /<EmbedManifest>false<\/EmbedManifest>/i);
});
