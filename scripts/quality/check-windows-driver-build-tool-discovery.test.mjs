import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('virtual driver build finds WDK tools from both bin and Tools trees', () => {
  const text = readFileSync('scripts/windows/build-virtual-driver.ps1', 'utf8');

  assert.match(text, /Windows Kits\\10\\bin/i);
  assert.match(text, /Windows Kits\\10\\Tools/i);
  assert.match(text, /Get-Command\s+\$Name/i);
  assert.match(text, /Inf2Cat\.exe/i);
});
