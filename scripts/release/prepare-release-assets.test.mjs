import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { prepareReleaseAssets } from './prepare-release-assets.mjs';

test('flattens workflow artifacts without duplicate release asset names', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-release-assets-'));
  const input = path.join(root, 'downloaded');
  const output = path.join(root, 'release');
  for (const variant of ['Voxveil-windows-standard', 'Voxveil-windows-pro-system']) {
    await mkdir(path.join(input, variant), { recursive: true });
    await writeFile(path.join(input, variant, 'Voxveil.exe'), variant);
    await writeFile(path.join(input, variant, 'manifest.json'), '{}');
  }

  const assets = await prepareReleaseAssets(input, output);
  const names = (await readdir(output)).sort();
  assert.equal(new Set(names).size, names.length);
  assert.equal(assets.length, 4);
  assert.ok(names.includes('Voxveil-windows-standard--Voxveil.exe'));
  assert.ok(names.includes('Voxveil-windows-pro-system--manifest.json'));
});
