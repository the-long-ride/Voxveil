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
