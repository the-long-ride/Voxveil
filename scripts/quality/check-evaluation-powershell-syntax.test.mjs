import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const evaluationDir = path.resolve('scripts/evaluation');
const scripts = readdirSync(evaluationDir)
  .filter((name) => name.endsWith('.ps1'))
  .sort()
  .map((name) => path.join(evaluationDir, name));

test('evaluation PowerShell scripts parse on the Windows release runner', { skip: process.platform !== 'win32' }, () => {
  const parseCommand = [
    '$tokens = $null',
    '$errors = $null',
    '[System.Management.Automation.Language.Parser]::ParseFile($env:VOXVEIL_SCRIPT, [ref]$tokens, [ref]$errors) | Out-Null',
    'if ($errors -and $errors.Count -gt 0) { $errors | ForEach-Object { Write-Error $_.Message }; exit 1 }',
  ].join('; ');

  assert.ok(scripts.length > 0, 'expected at least one evaluation PowerShell script');

  for (const script of scripts) {
    const result = spawnSync(
      'powershell.exe',
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
