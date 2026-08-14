import assert from 'node:assert/strict';
import { mkdtemp, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { missingLockfiles } from './require-lockfiles.mjs';

test('requires both npm and Cargo lockfiles', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-locks-'));
  assert.deepEqual(await missingLockfiles(root), ['package-lock.json', 'Cargo.lock']);
  await writeFile(path.join(root, 'package-lock.json'), '{}');
  await writeFile(path.join(root, 'Cargo.lock'), 'version = 4\n');
  assert.deepEqual(await missingLockfiles(root), []);
});
