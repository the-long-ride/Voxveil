import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const buildScript = () => readFileSync('scripts/windows/build-windows.ps1', 'utf8');

test('Windows package hashing does not depend on Get-FileHash availability', () => {
  const text = buildScript();
  assert.doesNotMatch(text, /\bGet-FileHash\b/i);
  assert.match(text, /function\s+Get-Sha256Hex\b/i);
  assert.match(text, /Security\.Cryptography\.SHA256\]::Create\(\)/i);
  assert.match(text, /\.ComputeHash\(/i);
  assert.match(text, /Get-Sha256Hex\s+\$file\.FullName/i);
});
