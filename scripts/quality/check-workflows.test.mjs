import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { auditWorkflows } from './check-workflows.mjs';

async function writeWorkflow(root, name, content = 'name: forbidden\n') {
  await mkdir(path.join(root, '.github/workflows'), { recursive: true });
  await writeFile(path.join(root, '.github/workflows', name), content);
}

test('accepts a repository with no workflow directory', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-free-'));
  assert.deepEqual(await auditWorkflows(root), []);
});

test('accepts an empty workflow directory', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-free-'));
  await mkdir(path.join(root, '.github/workflows'), { recursive: true });
  assert.deepEqual(await auditWorkflows(root), []);
});

test('rejects any YAML workflow file', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-free-'));
  await writeWorkflow(root, 'ci.yml');
  await writeWorkflow(root, 'manual-build.yaml');

  assert.deepEqual(await auditWorkflows(root), [
    'ci.yml is forbidden: Voxveil uses workflow-free local/manual builds',
    'manual-build.yaml is forbidden: Voxveil uses workflow-free local/manual builds',
  ]);
});

test('ignores non-workflow support files in the workflow directory', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-free-'));
  await writeWorkflow(root, 'README.md', '# no workflows here\n');
  assert.deepEqual(await auditWorkflows(root), []);
});
