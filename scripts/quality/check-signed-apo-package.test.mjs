import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

const expectedFiles = [
  'VoxveilApo.inf',
  'VoxveilApo.dll',
  'VoxveilApo.cat',
  'VoxveilApoExtension.inf',
  'VoxveilApoExtension.cat',
];

test('signed APO verifier checks exact package identities and catalog coverage', () => {
  const text = read('scripts/windows/verify-signed-apo-package.ps1');
  for (const name of expectedFiles) assert.match(text, new RegExp(name.replace('.', '\\.')));
  assert.match(text, /F3F2A99F-8FB7-4B88-949E-448BF8A05221/i);
  assert.match(text, /63E268CE-4CBC-48E0-BEB6-55103316F477/i);
  assert.match(text, /1D81E93D-AB81-473B-9E5E-94FAE8D2377F/i);
  assert.match(text, /verify\s+\/kp\s+\/v/i);
  assert.match(text, /verify\s+\/c/i);
  assert.match(text, /verify\s+\/pa\s+\/v\s+\$apoDll\.FullName/i);
  assert.match(text, /SignatureAttributes\.PETrust/i);
  assert.match(text, /Get-PeMachine/i);
  assert.match(text, /0x8664/i);
  assert.match(text, /InfVerif\.exe/i);
  assert.doesNotMatch(text, /TESTSIGNING|New-SelfSignedCertificate|signtool\s+sign/i);
});

test('signed APO stager copies only the verified production package', () => {
  const text = read('scripts/windows/stage-signed-apo-package.ps1');
  assert.match(text, /verify-signed-apo-package\.ps1/i);
  for (const name of expectedFiles) assert.match(text, new RegExp(name.replace('.', '\\.')));
  assert.match(text, /apo-verification\.json/i);
  assert.match(text, /apoInfSha256/);
  assert.match(text, /apoDllSha256/);
  assert.match(text, /apoCatalogSha256/);
  assert.match(text, /extensionInfSha256/);
  assert.match(text, /extensionCatalogSha256/);
  assert.doesNotMatch(text, /VoxveilDevelopment|\.cer\b|\.pfx\b/i);
});

test('production endpoint discovery requires the verified APO staging marker', () => {
  const text = read('crates/voxveil-windows-audio/src/discovery.rs');
  assert.match(text, /apo-verification\.json/i);
  assert.match(text, /1D81E93D-AB81-473B-9E5E-94FAE8D2377F/i);
  assert.match(text, /63E268CE-4CBC-48E0-BEB6-55103316F477/i);
});

test('production installer rehashes every signed APO artifact before installation', () => {
  const text = read('scripts/windows/install-system-audio-component.ps1');
  assert.match(text, /apo-verification\.json/i);
  assert.match(text, /Get-FileHash/i);
  for (const field of [
    'apoInfSha256',
    'apoDllSha256',
    'apoCatalogSha256',
    'extensionInfSha256',
    'extensionCatalogSha256',
  ]) {
    assert.match(text, new RegExp(field));
  }
});

test('Windows package build stages a signed APO only through the verifier path', () => {
  const text = read('scripts/windows/build-windows.ps1');
  assert.match(text, /VOXVEIL_SIGNED_APO_DIR/);
  assert.match(text, /stage-signed-apo-package\.ps1/i);
  assert.doesNotMatch(text, /Copy-Item\s+\$env:VOXVEIL_SIGNED_APO_DIR/i);
});
