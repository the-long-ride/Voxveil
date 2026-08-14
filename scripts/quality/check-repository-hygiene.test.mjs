import assert from 'node:assert/strict';
import { mkdtemp, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { scanRepositoryHygiene } from './check-repository-hygiene.mjs';

test('rejects generated TypeScript build metadata', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-hygiene-'));
  await writeFile(path.join(root, 'tsconfig.tsbuildinfo'), '{}');
  assert.match(scanRepositoryHygiene(root).join('\n'), /tsconfig\.tsbuildinfo/);
});
