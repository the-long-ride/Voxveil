import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdtemp, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';
import test from 'node:test';
import { scanRepositoryHygiene } from './check-repository-hygiene.mjs';

const execFileAsync = promisify(execFile);

async function initRepo() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-hygiene-'));
  await execFileAsync('git', ['init', '--quiet'], { cwd: root });
  await writeFile(path.join(root, '.gitignore'), '*.tsbuildinfo\n');
  await writeFile(path.join(root, 'tracked.txt'), 'tracked\n');
  await execFileAsync('git', ['add', '.gitignore', 'tracked.txt'], { cwd: root });
  return root;
}

test('ignores generated TypeScript build metadata when it is untracked', async () => {
  const root = await initRepo();
  await writeFile(path.join(root, 'tsconfig.tsbuildinfo'), '{}');
  assert.deepEqual(scanRepositoryHygiene(root), []);
});

test('rejects generated TypeScript build metadata when it is tracked', async () => {
  const root = await initRepo();
  await writeFile(path.join(root, 'tsconfig.tsbuildinfo'), '{}');
  await execFileAsync('git', ['add', '--force', 'tsconfig.tsbuildinfo'], { cwd: root });
  assert.match(scanRepositoryHygiene(root).join('\n'), /tsconfig\.tsbuildinfo/);
});
