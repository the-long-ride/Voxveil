import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { verifyProjectVersion } from './verify-version.mjs';

async function fixture(version) {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-version-'));
  await mkdir(path.join(root, 'ui'), { recursive: true });
  await mkdir(path.join(root, 'tauri'), { recursive: true });
  await writeFile(path.join(root, 'package.json'), JSON.stringify({ version }));
  await writeFile(path.join(root, 'ui/package.json'), JSON.stringify({ version }));
  await writeFile(path.join(root, 'tauri/package.json'), JSON.stringify({ version }));
  await writeFile(path.join(root, 'tauri/tauri.conf.json'), JSON.stringify({ version }));
  await writeFile(path.join(root, 'Cargo.toml'), `[workspace.package]\nversion = "${version}"\n`);
  return root;
}

test('accepts a tag matching all project version sources', async () => {
  const root = await fixture('1.2.3');
  assert.deepEqual(await verifyProjectVersion(root, 'v1.2.3'), []);
});

test('reports every mismatched version source', async () => {
  const root = await fixture('1.2.3');
  await writeFile(path.join(root, 'ui/package.json'), JSON.stringify({ version: '1.2.4' }));
  const errors = await verifyProjectVersion(root, 'v1.2.3');
  assert.match(errors.join('\n'), /ui\/package\.json.*1\.2\.4.*1\.2\.3/i);
});
