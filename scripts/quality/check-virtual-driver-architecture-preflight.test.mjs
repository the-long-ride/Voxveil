import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = () => readFileSync('scripts/windows/install-staged-virtual-driver.ps1', 'utf8');

test('virtual driver installer rejects a staged package for the wrong native architecture before creating the devnode', () => {
  const text = installer();

  assert.match(text, /function\s+Assert-StagedArchitecture/i);
  assert.match(text, /Get-CimInstance\s+Win32_Processor/i);
  assert.match(text, /Architecture\s*-eq\s*9|Architecture\s*-eq\s*12/i);
  assert.match(
    text,
    /Assert-StagedArchitecture\s+-PackageArchitecture\s+\(\[string\]\$verification\.architecture\)/i,
  );

  const manifest = text.search(/\$verification\s*=\s*Get-Content\s+\$manifestPath/i);
  const preflight = text.search(
    /Assert-StagedArchitecture\s+-PackageArchitecture\s+\(\[string\]\$verification\.architecture\)/i,
  );
  const ensure = text.search(/&\s*\$deviceHelper\s+ensure/i);

  assert.ok(manifest >= 0, 'installer must load verification.json before checking package architecture');
  assert.ok(preflight > manifest, 'package architecture must be checked after loading verification.json');
  assert.ok(ensure > preflight, 'package architecture must be rejected before ensuring the root devnode');
});
