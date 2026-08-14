import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { auditWorkflows } from './check-workflows.mjs';

async function writeWorkflow(root, name, content) {
  await mkdir(path.join(root, '.github/workflows'), { recursive: true });
  await writeFile(path.join(root, '.github/workflows', name), content);
}

test('accepts pinned actions and required Voxveil workflow triggers', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-'));
  const pin = '1234567890123456789012345678901234567890';
  await writeWorkflow(root, 'ci.yml', `on:\n  push:\n    branches: [master]\n  pull_request:\n    branches: [master]\npermissions:\n  contents: read\njobs:\n  gate:\n    steps:\n      - uses: actions/checkout@${pin}\n      - run: node scripts/ci/require-lockfiles.mjs\n      - run: npm ci --ignore-scripts\n      - run: npm run quality\n      - run: npm run coverage\n      - run: cargo llvm-cov --fail-under-lines 85\n      - run: cargo deny check\n`);
  await writeWorkflow(root, 'manual-build.yml', `on:\n  workflow_dispatch:\n    inputs:\n      ref: {}\n      platform: {}\n      edition: {}\n# windows linux macos android ios standard pro-system\n- run: node scripts/ci/require-lockfiles.mjs\n- run: npm ci --ignore-scripts\n- run: npm run quality\n- run: npm run coverage\n- run: cargo llvm-cov --fail-under-lines 85\n- run: cargo deny check\n- uses: actions/upload-artifact@${pin}\n`);
  await writeWorkflow(root, 'release.yml', `on:\n  push:\n    tags:\n      - 'v*.*.*'\n# windows linux macos android ios standard pro-system\n- run: node scripts/ci/require-lockfiles.mjs\n- run: npm ci --ignore-scripts\n- run: npm run quality\n- run: npm run coverage\n- run: cargo llvm-cov --fail-under-lines 85\n- run: cargo deny check\n- run: node scripts/release/verify-version.mjs v1.2.3\n- run: node scripts/release/generate-release-metadata.mjs\n- run: node scripts/release/prepare-release-assets.mjs\n- uses: actions/upload-artifact@${pin}\n`);
  assert.deepEqual(await auditWorkflows(root), []);
});

test('rejects floating action tags and dangerous pipe-to-shell commands', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-'));
  await writeWorkflow(root, 'ci.yml', `on: [push]\nsteps:\n  - uses: actions/checkout@v7\n  - run: curl https://example.invalid/install.sh | sh\n`);
  await writeWorkflow(root, 'manual-build.yml', 'on: workflow_dispatch\n');
  await writeWorkflow(root, 'release.yml', 'on: push\n');
  const errors = (await auditWorkflows(root)).join('\n');
  assert.match(errors, /immutable 40-character SHA/i);
  assert.match(errors, /pipe.*shell/i);
});

test('requires full coverage and metadata gates in manual and release workflows', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-'));
  const pin = '1234567890123456789012345678901234567890';
  await writeWorkflow(root, 'ci.yml', `push:\npull_request:\nmaster\ncontents: read\nnode scripts/ci/require-lockfiles.mjs\nnpm ci --ignore-scripts\nnpm run quality\nnpm run coverage\ncargo llvm-cov --fail-under-lines 85\ncargo deny check\n`);
  await writeWorkflow(root, 'manual-build.yml', `workflow_dispatch:\nref:\nplatform:\nedition:\nwindows linux macos android ios standard pro-system\nuses: actions/upload-artifact@${pin}\n`);
  await writeWorkflow(root, 'release.yml', `push:\ntags:\nv*.*.*\nwindows linux macos android ios standard pro-system\nuses: actions/upload-artifact@${pin}\n`);
  const errors = (await auditWorkflows(root)).join('\n');
  assert.match(errors, /manual-build\.yml must contain cargo llvm-cov/);
  assert.match(errors, /release\.yml must contain node scripts\/release\/generate-release-metadata\.mjs/);
});

test('release workflow stages unique asset names before publishing', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-'));
  const pin = '1234567890123456789012345678901234567890';
  await writeWorkflow(root, 'ci.yml', `push:\npull_request:\nmaster\ncontents: read\nnode scripts/ci/require-lockfiles.mjs\nnpm ci --ignore-scripts\nnpm run quality\nnpm run coverage\ncargo llvm-cov --fail-under-lines 85\ncargo deny check\n`);
  await writeWorkflow(root, 'manual-build.yml', `workflow_dispatch:\nref:\nplatform:\nedition:\nwindows linux macos android ios standard pro-system\nnode scripts/ci/require-lockfiles.mjs\nnpm ci --ignore-scripts\nnpm run quality\nnpm run coverage\ncargo llvm-cov --fail-under-lines 85\ncargo deny check\nuses: actions/upload-artifact@${pin}\n`);
  await writeWorkflow(root, 'release.yml', `push:\ntags:\nv*.*.*\nwindows linux macos android ios standard pro-system\nnode scripts/ci/require-lockfiles.mjs\nnpm ci --ignore-scripts\nnpm run quality\nnpm run coverage\ncargo llvm-cov --fail-under-lines 85\ncargo deny check\nnode scripts/release/verify-version.mjs v1.2.3\nnode scripts/release/generate-release-metadata.mjs\nuses: actions/upload-artifact@${pin}\n`);
  const errors = (await auditWorkflows(root)).join('\n');
  assert.match(errors, /prepare-release-assets\.mjs/);
});

test('release workflow verifies tag and manifest versions agree', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-workflow-'));
  const pin = '1234567890123456789012345678901234567890';
  await writeWorkflow(root, 'ci.yml', `push:\npull_request:\nmaster\ncontents: read\nnode scripts/ci/require-lockfiles.mjs\nnpm ci --ignore-scripts\nnpm run quality\nnpm run coverage\ncargo llvm-cov --fail-under-lines 85\ncargo deny check\n`);
  await writeWorkflow(root, 'manual-build.yml', `workflow_dispatch:\nref:\nplatform:\nedition:\nwindows linux macos android ios standard pro-system\nnode scripts/ci/require-lockfiles.mjs\nnpm ci --ignore-scripts\nnpm run quality\nnpm run coverage\ncargo llvm-cov --fail-under-lines 85\ncargo deny check\nuses: actions/upload-artifact@${pin}\n`);
  await writeWorkflow(root, 'release.yml', `push:\ntags:\nv*.*.*\nwindows linux macos android ios standard pro-system\nnode scripts/ci/require-lockfiles.mjs\nnpm ci --ignore-scripts\nnpm run quality\nnpm run coverage\ncargo llvm-cov --fail-under-lines 85\ncargo deny check\nnode scripts/release/generate-release-metadata.mjs\nnode scripts/release/prepare-release-assets.mjs\nuses: actions/upload-artifact@${pin}\n`);
  const errors = (await auditWorkflows(root)).join('\n');
  assert.match(errors, /verify-version\.mjs/);
});
