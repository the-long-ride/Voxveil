import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('APO extension template carries fixed CAPX and servicing identities', () => {
  const text = read('native/windows/package/VoxveilApoExtension.inf.template');
  assert.match(text, /ExtensionId\s*=\s*\{1D81E93D-AB81-473B-9E5E-94FAE8D2377F\}/i);
  assert.match(text, /VOXVEIL_APO_CONTEXT\s*=\s*"\{63E268CE-4CBC-48E0-BEB6-55103316F477\}"/i);
  assert.match(text, /@@FX_PROPERTY_ASSOCIATION@@/);
  assert.doesNotMatch(text, /@@EXTENSION_ID@@/);
});

test('extension generator preserves the committed ExtensionId across versions', () => {
  const text = read('scripts/windows/new-apo-extension-inf.ps1');
  assert.doesNotMatch(text, /NewGuid|New-Guid|@@EXTENSION_ID@@/i);
  assert.match(text, /1D81E93D-AB81-473B-9E5E-94FAE8D2377F/i);
});

test('extension generator has explicit CAPX and legacy modes', () => {
  const text = read('scripts/windows/new-apo-extension-inf.ps1');
  assert.match(text, /ValidateSet\('Capx',\s*'Legacy'\)/);
  assert.match(text, /FxPropertyMode\s*=\s*'Capx'/);
  assert.match(text, /HKR,FX\\0\\%VOXVEIL_APO_CONTEXT%,%PKEY_FX_Association%,,%KSNODETYPE_ANY%/i);
  assert.match(text, /HKR,FX\\0\\%VOXVEIL_APO_CONTEXT%\\User,,,/i);
  assert.match(text, /HKR,FX\\0,%PKEY_FX_Association%,,%KSNODETYPE_ANY%/i);
});

test('production installer requires CAPX binding, extension lineage, and reserves runtime FX mutation for test signing', () => {
  const text = read('scripts/windows/install-system-audio-component.ps1');
  assert.match(text, /CAPX production/i);
  assert.match(text, /VOXVEIL_APO_CONTEXT/i);
  assert.match(text, /ExtensionId[^\r\n]*1D81E93D-AB81-473B-9E5E-94FAE8D2377F/i);
  assert.match(text, /servicing lineage|extension lineage/i);
  assert.match(text, /\$TestSign\s+-and\s+\$runtimeBound/);
  assert.doesNotMatch(text, /if \(\$runtimeBound\) \{\s*if \(-not \(Test-Path \$control/s);
});
