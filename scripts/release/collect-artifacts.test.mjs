import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { collectArtifacts } from './collect-artifacts.mjs';

test('collects platform artifacts and writes reproducible SHA-256 metadata', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-artifacts-'));
  const source = path.join(root, 'target/release/bundle/msi');
  await mkdir(source, { recursive: true });
  await writeFile(path.join(source, 'Voxveil_0.1.0_x64.msi'), 'voxveil');

  const result = await collectArtifacts(root, 'windows', 'standard');
  assert.equal(result.files.length, 1);
  const sums = await readFile(path.join(result.outputDir, 'SHA256SUMS'), 'utf8');
  assert.match(sums, /^[a-f0-9]{64}  Voxveil_0\.1\.0_x64\.msi\n$/);
  const manifest = JSON.parse(await readFile(path.join(result.outputDir, 'manifest.json'), 'utf8'));
  assert.equal(manifest.platform, 'windows');
  assert.equal(manifest.edition, 'standard');
  assert.equal(manifest.files[0].name, 'Voxveil_0.1.0_x64.msi');
});

test('windows collection includes the raw voxveil executable beside installers', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-artifacts-'));
  const installerDir = path.join(root, 'target/release/bundle/msi');
  const releaseDir = path.join(root, 'target/release');
  await mkdir(installerDir, { recursive: true });
  await writeFile(path.join(installerDir, 'Voxveil_0.1.0_x64.msi'), 'installer');
  await writeFile(path.join(releaseDir, 'voxveil.exe'), 'portable');

  const result = await collectArtifacts(root, 'windows', 'standard');
  assert.deepEqual(
    result.files.map((file) => file.name).sort(),
    ['Voxveil_0.1.0_x64.msi', 'voxveil.exe'],
  );
});

test('windows pro-system collection requires and preserves the system-audio package', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-artifacts-'));
  const releaseDir = path.join(root, 'target/release');
  const componentDir = path.join(root, 'target/system-audio');
  await mkdir(releaseDir, { recursive: true });
  await mkdir(componentDir, { recursive: true });
  await writeFile(path.join(releaseDir, 'voxveil.exe'), 'portable');
  for (const file of ['VoxveilApo.dll', 'install.ps1', 'uninstall.ps1', 'README.txt']) {
    await writeFile(path.join(componentDir, file), file);
  }

  const result = await collectArtifacts(root, 'windows', 'pro-system');
  assert.deepEqual(
    result.files.map((file) => file.name).sort(),
    [
      'system-audio/README.txt',
      'system-audio/VoxveilApo.dll',
      'system-audio/install.ps1',
      'system-audio/uninstall.ps1',
      'voxveil.exe',
    ],
  );
  const sums = await readFile(path.join(result.outputDir, 'SHA256SUMS'), 'utf8');
  assert.match(sums, /system-audio\/VoxveilApo\.dll/);
});

test('windows pro-system collection rejects a missing system-audio package', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-artifacts-'));
  const releaseDir = path.join(root, 'target/release');
  await mkdir(releaseDir, { recursive: true });
  await writeFile(path.join(releaseDir, 'voxveil.exe'), 'portable');
  await assert.rejects(
    () => collectArtifacts(root, 'windows', 'pro-system'),
    /system-audio package/i,
  );
});

test('non-Windows collection does not include the raw Windows executable', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-artifacts-'));
  const linuxDir = path.join(root, 'target/release/bundle/deb');
  const releaseDir = path.join(root, 'target/release');
  await mkdir(linuxDir, { recursive: true });
  await writeFile(path.join(linuxDir, 'voxveil.deb'), 'linux');
  await writeFile(path.join(releaseDir, 'voxveil.exe'), 'windows');

  const result = await collectArtifacts(root, 'linux', 'standard');
  assert.deepEqual(result.files.map((file) => file.name), ['voxveil.deb']);
});

test('fails instead of uploading an empty build', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-artifacts-'));
  await assert.rejects(() => collectArtifacts(root, 'linux', 'standard'), /no build artifacts/i);
});
