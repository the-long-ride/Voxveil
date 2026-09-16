import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

test('manual Windows build runs the full repository verification gate', async () => {
  const source = await readFile('scripts/windows/build-windows.ps1', 'utf8');

  for (const command of [
    'cargo test --workspace',
    'npm test',
    'npm run typecheck',
    'npm run quality',
  ]) {
    assert.match(source, new RegExp(command.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }
});
