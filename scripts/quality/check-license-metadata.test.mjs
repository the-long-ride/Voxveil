import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { auditDependencyMetadata } from './check-license-metadata.mjs';

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
