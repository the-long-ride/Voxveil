import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { auditDependencyMetadata, auditThirdPartyMetadata } from './check-license-metadata.mjs';

const WINDOWS_DRIVER_SAMPLES_REVISION = '67d81f217bc01edf7a4320e4911c11065635acfa';
const WINDOWS_DRIVER_SAMPLES_SYSVAD_TREE = '6fa502f5bfb3de1395a6c9ffe71e322fd9e28926';

async function fixture(packages, allowlist) {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-license-'));
  await mkdir(path.join(root, 'docs/specs/security'), { recursive: true });
  await writeFile(path.join(root, 'package.json'), JSON.stringify(packages));
  await writeFile(
    path.join(root, 'docs/specs/security/dependency-allowlist.json'),
    JSON.stringify({ schemaVersion: 1, packages: allowlist }),
  );
  return root;
}

async function writeDriverSampleMetadata(
  root,
  revision = WINDOWS_DRIVER_SAMPLES_REVISION,
  sysvadTree = WINDOWS_DRIVER_SAMPLES_SYSVAD_TREE,
) {
  const base = path.join(root, 'third_party/microsoft/windows-driver-samples');
  await mkdir(base, { recursive: true });
  await writeFile(path.join(base, 'SOURCE_REVISION'), `${revision}\n`);
  await writeFile(path.join(base, 'SYSVAD_TREE_SHA'), `${sysvadTree}\n`);
  await writeFile(
    path.join(base, 'README.voxveil.md'),
    '# Microsoft Windows Driver Samples provenance\n\n' +
      'Upstream: https://github.com/microsoft/Windows-driver-samples\n' +
      'Imported area: audio/sysvad\n' +
      `Pinned audio/sysvad tree: ${sysvadTree}\n`,
  );
  await writeFile(path.join(base, 'LICENSE.txt'), 'The Microsoft Public License (MS-PL)\n');
}

test('accepts exact dependencies present in the approved allowlist', async () => {
  const root = await fixture(
    { dependencies: { react: '19.2.8' } },
    { react: { version: '19.2.8', license: 'MIT', commercialUse: true } },
  );
  assert.deepEqual(await auditDependencyMetadata(root), []);
});

test('rejects undeclared dependencies', async () => {
  const root = await fixture(
    { dependencies: { react: '19.2.8', mystery: '1.0.0' } },
    { react: { version: '19.2.8', license: 'MIT', commercialUse: true } },
  );
  assert.match((await auditDependencyMetadata(root)).join('\n'), /mystery.*not allowlisted/i);
});

test('rejects version drift and non-commercial approvals', async () => {
  const root = await fixture(
    { dependencies: { react: '19.2.9', unsafe: '1.0.0' } },
    {
      react: { version: '19.2.8', license: 'MIT', commercialUse: true },
      unsafe: { version: '1.0.0', license: 'Unknown', commercialUse: false },
    },
  );
  const errors = (await auditDependencyMetadata(root)).join('\n');
  assert.match(errors, /react.*19\.2\.9.*19\.2\.8/i);
  assert.match(errors, /unsafe.*commercial use/i);
});

test('audits wildcard workspace package manifests', async () => {
  const root = await fixture(
    { workspaces: ['packages/*'] },
    {},
  );
  await mkdir(path.join(root, 'packages', 'plugin'), { recursive: true });
  await writeFile(path.join(root, 'packages', 'plugin', 'package.json'), JSON.stringify({ dependencies: { mystery: '1.0.0' } }));
  const errors = (await auditDependencyMetadata(root)).join('\n');
  assert.match(errors, /mystery.*not allowlisted/i);
});

test('accepts pinned Microsoft Windows Driver Samples provenance', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-third-party-'));
  await writeDriverSampleMetadata(root);
  assert.deepEqual(await auditThirdPartyMetadata(root), []);
});

test('rejects Windows Driver Samples revision drift', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-third-party-'));
  await writeDriverSampleMetadata(root, 'deadbeef');
  assert.match(
    (await auditThirdPartyMetadata(root)).join('\n'),
    /windows-driver-samples.*revision.*67d81f217bc01edf7a4320e4911c11065635acfa/i,
  );
});

test('rejects SysVAD subtree identity drift', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-third-party-'));
  await writeDriverSampleMetadata(root, WINDOWS_DRIVER_SAMPLES_REVISION, 'deadbeef');
  assert.match(
    (await auditThirdPartyMetadata(root)).join('\n'),
    /audio\/sysvad.*tree.*6fa502f5bfb3de1395a6c9ffe71e322fd9e28926/i,
  );
});
