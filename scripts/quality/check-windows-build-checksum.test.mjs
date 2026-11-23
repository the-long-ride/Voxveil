import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';

const buildScript = () => readFileSync('scripts/windows/build-windows.ps1', 'utf8');
const scripts = readdirSync('scripts/windows')
  .filter((name) => name.endsWith('.ps1'))
  .map((name) => path.join('scripts/windows', name));
const hashingScripts = scripts.filter((script) =>
  /function\s+Get-Sha256Hex\b/i.test(readFileSync(script, 'utf8')));

test('Windows package hashing does not depend on Get-FileHash availability', () => {
  const text = buildScript();
  assert.doesNotMatch(text, /\bGet-FileHash\b/i);
  assert.match(text, /function\s+Get-Sha256Hex\b/i);
  assert.match(text, /Security\.Cryptography\.SHA256\]::Create\(\)/i);
  assert.match(text, /\.ComputeHash\(/i);
  assert.match(text, /Get-Sha256Hex\s+\$file\.FullName/i);
});

test('all Windows scripts avoid module-dependent hashing and ZIP cmdlets', () => {
  for (const script of scripts) {
    assert.doesNotMatch(readFileSync(script, 'utf8'), /\b(?:Get-FileHash|Expand-Archive|Compress-Archive)\b/i, script);
  }
  for (const script of hashingScripts) {
    const text = readFileSync(script, 'utf8');
    assert.match(text, /Security\.Cryptography\.SHA256\]::Create\(\)/i, script);
    assert.match(text, /\.ComputeHash\(\$stream\)/i, script);
    assert.match(text, /\.Replace\('-', ''\)\.ToLowerInvariant\(\)/i, script);
    assert.match(text, /finally\s*\{\s*\$sha256\.Dispose\(\)/i, script);
    assert.match(text, /finally\s*\{\s*\$stream\.Dispose\(\)/i, script);
  }
});

test('Windows module-dependent signature, CIM, DISM and certificate calls use trusted module paths', () => {
  for (const script of scripts) {
    const text = readFileSync(script, 'utf8');
    if (!/\b(?:Get-AuthenticodeSignature|Get-CimInstance|Get-WindowsDriver|New-SelfSignedCertificate|Export-Certificate)\b/.test(text)) continue;
    if (/\$env:PSModulePath\s*=\s*\$trustedModulePath/.test(text)) {
      assert.match(text, /\$trustedSystemDirectoryForModules\s*=\s*\[Environment\]::SystemDirectory/, script);
      assert.match(text, /WindowsPowerShell\\v1\.0\\Modules/, script);
    } else {
      assert.match(text, /Import-Module \(Join-Path \$PSHOME 'Modules\\Microsoft\.PowerShell\.Security\\Microsoft\.PowerShell\.Security\.psd1'\) -ErrorAction Stop/, script);
      assert.doesNotMatch(text, /\b(?:Get-CimInstance|Get-WindowsDriver|New-SelfSignedCertificate|Export-Certificate)\b/, script);
    }
  }
});

test('Windows SHA-256 helpers hash literal paths and empty files without module autoloading', { skip: process.platform !== 'win32' }, () => {
  const directory = mkdtempSync(path.join(tmpdir(), 'voxveil-sha256-'));
  try {
    const payload = Buffer.from([0, 255, 1, 128, 10]);
    const file = path.join(directory, 'payload [literal] é.bin');
    const empty = path.join(directory, 'empty.bin');
    writeFileSync(file, payload);
    writeFileSync(empty, '');
    const command = `
      $ErrorActionPreference = 'Stop'
      $env:PSModulePath = ''
      $PSModuleAutoLoadingPreference = 'None'
      function Get-FileHash { throw 'Module-dependent hashing must not be called.' }
      $tokens = $null; $errors = $null
      $ast = [System.Management.Automation.Language.Parser]::ParseFile($env:VOXVEIL_SCRIPT, [ref]$tokens, [ref]$errors)
      if ($errors.Count) { throw 'PowerShell script failed to parse.' }
      $helper = $ast.Find({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-Sha256Hex' }, $true)
      if (-not $helper) { throw 'SHA-256 helper was not found.' }
      . ([scriptblock]::Create($helper.Extent.Text))
      foreach ($case in @(@{ Path = $env:VOXVEIL_PAYLOAD; Hash = $env:VOXVEIL_EXPECTED }, @{ Path = $env:VOXVEIL_EMPTY; Hash = $env:VOXVEIL_EMPTY_HASH })) {
        if ((Get-Sha256Hex $case.Path) -cne $case.Hash) { throw 'SHA-256 did not match lowercase known answer.' }
        $exclusive = [IO.File]::Open($case.Path, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
        $exclusive.Dispose()
      }
      $failed = $false
      try { $null = Get-Sha256Hex ($env:VOXVEIL_PAYLOAD + '.missing') } catch { $failed = $true }
      if (-not $failed) { throw 'Hashing a missing file did not fail.' }
    `;
    for (const script of hashingScripts) {
      const result = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', command], {
        encoding: 'utf8',
        env: { ...process.env, VOXVEIL_SCRIPT: path.resolve(script), VOXVEIL_PAYLOAD: file,
          VOXVEIL_EXPECTED: createHash('sha256').update(payload).digest('hex'), VOXVEIL_EMPTY: empty,
          VOXVEIL_EMPTY_HASH: createHash('sha256').update('').digest('hex') },
      });
      assert.equal(result.status, 0, `${script}: ${result.error ?? ''}\n${result.stdout}\n${result.stderr}`);
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('signature verifiers load their own Security module with an empty module search path', { skip: process.platform !== 'win32' }, () => {
  for (const script of ['scripts/windows/verify-signed-apo-package.ps1', 'scripts/windows/verify-signed-virtual-driver.ps1']) {
    const importCommand = readFileSync(script, 'utf8').match(/^Import-Module[^\r\n]+/m)?.[0];
    assert.ok(importCommand, script);
    const command = `$ErrorActionPreference = 'Stop'; $env:PSModulePath = ''; $PSModuleAutoLoadingPreference = 'None'; ${importCommand}; $signature = Get-Command Get-AuthenticodeSignature -ErrorAction Stop; if ($signature.Source -ne 'Microsoft.PowerShell.Security') { throw 'Wrong signature module.' }`;
    const result = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', command], { encoding: 'utf8' });
    assert.equal(result.status, 0, `${script}: ${result.error ?? ''}\n${result.stdout}\n${result.stderr}`);
  }
});
