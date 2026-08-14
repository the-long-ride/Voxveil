import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { walkFilesSync } from './walk-files.mjs';

test('walkFilesSync returns relative files and skips ignored directories', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-walk-'));
  await mkdir(path.join(root, 'src'), { recursive: true });
  await mkdir(path.join(root, 'node_modules', 'pkg'), { recursive: true });
  await writeFile(path.join(root, 'src', 'a.ts'), 'a');
  await writeFile(path.join(root, 'node_modules', 'pkg', 'hidden.ts'), 'x');
  assert.deepEqual(walkFilesSync(root, new Set(['node_modules'])), ['src/a.ts']);
});

test('walkFiles returns sorted absolute files and can ignore a missing root', async () => {
  const { walkFiles } = await import('./walk-files.mjs');
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-walk-async-'));
  await mkdir(path.join(root, 'nested'), { recursive: true });
  await writeFile(path.join(root, 'z.txt'), 'z');
  await writeFile(path.join(root, 'nested', 'a.txt'), 'a');
  assert.deepEqual(await walkFiles(root), [path.join(root, 'nested', 'a.txt'), path.join(root, 'z.txt')]);
  assert.deepEqual(await walkFiles(path.join(root, 'missing'), { ignoreMissing: true }), []);
});
