import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('virtual driver build pins Visual Studio and WDK discovery to OS-known Program Files', () => {
  const text = readFileSync('scripts/windows/build-virtual-driver.ps1', 'utf8');

  assert.match(text, /GetFolderPath\(\[Environment\+SpecialFolder\]::ProgramFilesX86\)/i);
  assert.match(text, /Microsoft Visual Studio\\Installer\\vswhere\.exe/i);
  assert.match(text, /Windows Kits\\10\\bin/i);
  assert.match(text, /Windows Kits\\10\\Tools/i);
  assert.doesNotMatch(text, /\$env:ProgramFiles\(x86\)/i);
  assert.doesNotMatch(text, /Get-Command\s+(?:\$Name|msbuild\.exe)/i);
  assert.match(text, /Inf2Cat\.exe/i);
});
