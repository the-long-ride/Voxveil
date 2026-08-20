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
