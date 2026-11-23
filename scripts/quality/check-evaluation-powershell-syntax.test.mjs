import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const powershell = process.platform === 'win32' ? 'powershell.exe' : process.env.VOXVEIL_TEST_POWERSHELL;
const scripts = ['scripts/evaluation', 'scripts/windows'].flatMap((directory) =>
  readdirSync(directory)
    .filter((name) => name.endsWith('.ps1'))
    .sort()
    .map((name) => path.resolve(directory, name)));

test('build and evaluation PowerShell scripts parse', { skip: !powershell }, () => {
  const parseCommand = [
    '$tokens = $null',
    '$errors = $null',
    '[System.Management.Automation.Language.Parser]::ParseFile($env:VOXVEIL_SCRIPT, [ref]$tokens, [ref]$errors) | Out-Null',
    'if ($errors -and $errors.Count -gt 0) { foreach ($error in $errors) { [Console]::Error.WriteLine(("{0}:{1}: {2}" -f $error.Extent.StartLineNumber, $error.Extent.StartColumnNumber, $error.Message)) }; exit 1 }',
  ].join('; ');

  assert.ok(scripts.length > 0, 'expected at least one evaluation PowerShell script');

  for (const script of scripts) {
    const result = spawnSync(
      powershell,
      ['-NoProfile', '-NonInteractive', '-Command', parseCommand],
      {
        encoding: 'utf8',
        env: { ...process.env, VOXVEIL_SCRIPT: script },
      },
    );

    assert.equal(
      result.status,
      0,
      `${path.relative(process.cwd(), script)} failed PowerShell parsing:\n${result.stdout}\n${result.stderr}`,
    );
  }
});
