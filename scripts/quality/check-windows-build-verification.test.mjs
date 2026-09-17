import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

test('manual Windows build runs the full repository verification gate', async () => {
  const packageJson = JSON.parse(await readFile('package.json', 'utf8'));
  const command = packageJson.scripts?.['build:windows'] ?? '';

  for (const expected of [
    'npm ci --ignore-scripts --no-fund --no-audit',
    'cargo test --workspace',
    'npm test',
    'npm run typecheck',
    'npm run quality',
    'npm run build:windows-driver:x64',
    '-SkipNpmInstall',
  ]) {
    assert.match(command, new RegExp(expected.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }
  assert.doesNotMatch(command, /-SkipTests\b/);
});

test('manual Windows build preserves the validated unsigned driver submission separately', async () => {
  const workflow = await readFile('.github/workflows/manual-build.yml', 'utf8');
  assert.match(workflow, /name:\s*Voxveil-windows-driver-submission-\$\{\{\s*github\.sha\s*\}\}/);
  assert.match(workflow, /path:\s*native\/windows\/driver\/out\/x64\/submission\/?/);
  assert.match(workflow, /uses:\s*actions\/upload-artifact@v6/);
});
