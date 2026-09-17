import assert from 'node:assert/strict';
import test from 'node:test';
import { readFile } from 'node:fs/promises';

const buildScript = await readFile(
  new URL('../windows/build-windows.ps1', import.meta.url),
  'utf8',
);

test('Windows package checksum paths work in Windows PowerShell 5.1', () => {
  assert.doesNotMatch(buildScript, /\[IO\.Path\]::GetRelativePath\s*\(/);
  assert.match(buildScript, /function Get-RelativePackagePath/);
  assert.match(buildScript, /StringComparison\]::OrdinalIgnoreCase/);
});

test('Windows package fails if a required system-audio script is missing', () => {
  const start = buildScript.indexOf('foreach ($script in @(');
  const end = buildScript.indexOf('$signedApoDir =', start);
  const staging = start >= 0 && end > start ? buildScript.slice(start, end) : '';

  for (const script of [
    'discover-system-audio-endpoints.ps1',
    'new-apo-extension-inf.ps1',
    'install-system-audio-component.ps1',
    'uninstall-system-audio-component.ps1',
    'probe-apo-capx.ps1',
    'install-staged-virtual-driver.ps1',
    'uninstall-staged-virtual-driver.ps1',
  ]) {
    assert.match(staging, new RegExp(script.replaceAll('.', '\\.')));
  }

  assert.match(staging, /if \(-not \(Test-Path \$source -PathType Leaf\)\) \{\s*throw/i);
  assert.match(staging, /Copy-Item \$source \$systemAudio/);
  assert.doesNotMatch(staging, /if \(Test-Path \$source -PathType Leaf\) \{\s*Copy-Item/i);
});
