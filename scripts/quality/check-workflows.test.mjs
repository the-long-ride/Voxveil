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

const validManualWorkflow = `name: Manual Build
on:
  workflow_dispatch:
    inputs:
      windows:
        type: boolean
        default: true
      linux:
        type: boolean
        default: false
      macos:
        type: boolean
        default: false
jobs: {}
`;

const verificationPushWorkflow = validManualWorkflow.replace(
  '  workflow_dispatch:\n',
  '  push:\n    branches:\n      - feat/windows-signed-audio-paths\n  workflow_dispatch:\n',
);

const realisticVerificationWorkflow = `name: Manual Build

on:
  push:
    branches:
      - feat/windows-signed-audio-paths
  workflow_dispatch:
    inputs:
      windows:
        description: Build the Windows x64 package
        required: false
        type: boolean
        default: true
      linux:
        description: Build Linux bundles
        required: false
        type: boolean
        default: false
      macos:
        description: Build macOS bundles
        required: false
        type: boolean
        default: false

permissions:
  contents: read

jobs: {}
`;

test('accepts a repository with no workflow directory', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  assert.deepEqual(await auditWorkflows(root), []);
});

test('accepts an empty workflow directory', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  await mkdir(path.join(root, '.github/workflows'), { recursive: true });
  assert.deepEqual(await auditWorkflows(root), []);
});

test('accepts the single manual-build workflow with the required platform toggles', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  await writeWorkflow(root, 'manual-build.yml', validManualWorkflow);
  assert.deepEqual(await auditWorkflows(root), []);
});

test('accepts the exact feature-branch verification push alongside manual dispatch', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  await writeWorkflow(root, 'manual-build.yml', verificationPushWorkflow);
  assert.deepEqual(await auditWorkflows(root), []);
});

test('accepts the exact verification push in the real workflow shape and CRLF form', async () => {
  for (const content of [
    realisticVerificationWorkflow,
    realisticVerificationWorkflow.replaceAll('\n', '\r\n'),
  ]) {
    const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
    await writeWorkflow(root, 'manual-build.yml', content);
    assert.deepEqual(await auditWorkflows(root), []);
  }
});

test('rejects any additional YAML workflow file', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  await writeWorkflow(root, 'manual-build.yml', validManualWorkflow);
  await writeWorkflow(root, 'ci.yml', 'name: CI\non: push\n');

  assert.deepEqual(await auditWorkflows(root), [
    'ci.yml is forbidden: only manual-build.yml is allowed',
  ]);
});

test('rejects automatic triggers in manual-build.yml', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  const automaticWorkflow = validManualWorkflow.replace(
    '  workflow_dispatch:\n',
    '  push:\n  workflow_dispatch:\n',
  );
  await writeWorkflow(root, 'manual-build.yml', automaticWorkflow);

  assert.deepEqual(await auditWorkflows(root), [
    'manual-build.yml must be workflow_dispatch-only except for the exact feat/windows-signed-audio-paths verification push',
  ]);
});

test('rejects a verification push scoped to any other branch', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  const wrongBranch = verificationPushWorkflow.replace(
    '      - feat/windows-signed-audio-paths\n',
    '      - main\n',
  );
  await writeWorkflow(root, 'manual-build.yml', wrongBranch);
  assert.deepEqual(await auditWorkflows(root), [
    'manual-build.yml must be workflow_dispatch-only except for the exact feat/windows-signed-audio-paths verification push',
  ]);
});

test('rejects an additional branch from the verification push', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  const broadPush = verificationPushWorkflow.replace(
    '      - feat/windows-signed-audio-paths\n',
    '      - feat/windows-signed-audio-paths\n      - main\n',
  );
  await writeWorkflow(root, 'manual-build.yml', broadPush);
  assert.deepEqual(await auditWorkflows(root), [
    'manual-build.yml must be workflow_dispatch-only except for the exact feat/windows-signed-audio-paths verification push',
  ]);
});

test('rejects missing or incorrect manual platform toggle defaults', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  await writeWorkflow(root, 'manual-build.yml', `name: Manual Build
on:
  workflow_dispatch:
    inputs:
      windows:
        type: boolean
        default: false
      linux:
        type: boolean
        default: false
jobs: {}
`);

  assert.deepEqual(await auditWorkflows(root), [
    'manual-build.yml must define windows as boolean with default true',
    'manual-build.yml must define macos as boolean with default false',
  ]);
});

test('ignores non-workflow support files in the workflow directory', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflows-'));
  await writeWorkflow(root, 'README.md', '# manual workflow support\n');
  assert.deepEqual(await auditWorkflows(root), []);
});
