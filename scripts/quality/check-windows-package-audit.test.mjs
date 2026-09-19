import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { auditWindowsPackage } from '../evaluation/audit-windows-package.mjs';

const commit = 'a'.repeat(40);
const H = (value) => createHash('sha256').update(value).digest('hex');

async function listFiles(root, relative = '') {
  const entries = await readdir(path.join(root, relative), { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const child = relative ? path.posix.join(relative, entry.name) : entry.name;
    if (entry.isDirectory()) files.push(...await listFiles(root, child));
    else if (entry.isFile()) files.push(child);
  }
  return files.sort();
}

async function writeCompletePackage(root) {
  const apoDir = path.join(root, 'system-audio');
  const driverDir = path.join(apoDir, 'virtual-driver');
  await mkdir(driverDir, { recursive: true });

  const apo = JSON.stringify({ voxveilCommit: commit }, null, 2) + '\n';
  const driver = JSON.stringify({ voxveilCommit: commit, releaseChannel: 'retail' }, null, 2) + '\n';
  await writeFile(path.join(apoDir, 'apo-verification.json'), apo);
  await writeFile(path.join(driverDir, 'verification.json'), driver);
  await writeFile(path.join(root, 'voxveil.exe'), 'test executable bytes');

  const manifest = {
    schemaVersion: 1,
    voxveilCommit: commit,
    architecture: 'x64',
    signedApo: {
      present: true,
      verificationSha256: H(apo),
    },
    signedVirtualDriver: {
      present: true,
      releaseChannel: 'retail',
      verificationSha256: H(driver),
    },
    packageFilesHashedBy: 'SHA256SUMS.txt',
  };
  await writeFile(path.join(root, 'release-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');

  const files = (await listFiles(root)).filter((file) => file !== 'SHA256SUMS.txt');
  const lines = [];
  for (const file of files) {
    const bytes = await readFile(path.join(root, ...file.split('/')));
    lines.push(`${createHash('sha256').update(bytes).digest('hex')}  ${file}`);
  }
  await writeFile(path.join(root, 'SHA256SUMS.txt'), lines.join('\n') + '\n', 'ascii');
}

test('Windows package audit binds exact commit, signed component manifests, and every package file checksum', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-package-audit-'));
  try {
    await writeCompletePackage(root);
    const report = await auditWindowsPackage({
      packageRoot: root,
      commit,
      architecture: 'x64',
      releaseChannel: 'retail',
    });
    assert.equal(report.status, 'complete', report.issues.join('\n'));
    assert.equal(report.filesChecked, 4);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('Windows package audit fails closed after package byte tampering', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-package-audit-'));
  try {
    await writeCompletePackage(root);
    await writeFile(path.join(root, 'voxveil.exe'), 'tampered bytes');
    const report = await auditWindowsPackage({
      packageRoot: root,
      commit,
      architecture: 'x64',
      releaseChannel: 'retail',
    });
    assert.equal(report.status, 'incomplete');
    assert.ok(report.issues.some((issue) => /hash mismatch for voxveil\.exe/i.test(issue)));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('Windows package audit accepts PowerShell 5.1 UTF-8 BOM JSON manifests', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-package-audit-'));
  try {
    await writeCompletePackage(root);
    for (const file of [
      'release-manifest.json',
      'system-audio/apo-verification.json',
      'system-audio/virtual-driver/verification.json',
    ]) {
      const target = path.join(root, ...file.split('/'));
      const bytes = await readFile(target);
      await writeFile(target, Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), bytes]));
    }

    const files = (await listFiles(root)).filter((file) => file !== 'SHA256SUMS.txt');
    const lines = [];
    for (const file of files) {
      const bytes = await readFile(path.join(root, ...file.split('/')));
      lines.push(`${createHash('sha256').update(bytes).digest('hex')}  ${file}`);
    }
    await writeFile(path.join(root, 'SHA256SUMS.txt'), lines.join('\n') + '\n', 'ascii');

    const releaseBytes = await readFile(path.join(root, 'release-manifest.json'));
    const release = JSON.parse(releaseBytes.toString('utf8').replace(/^\uFEFF/, ''));
    const apoBytes = await readFile(path.join(root, 'system-audio', 'apo-verification.json'));
    const driverBytes = await readFile(path.join(root, 'system-audio', 'virtual-driver', 'verification.json'));
    release.signedApo.verificationSha256 = createHash('sha256').update(apoBytes).digest('hex');
    release.signedVirtualDriver.verificationSha256 = createHash('sha256').update(driverBytes).digest('hex');
    await writeFile(path.join(root, 'release-manifest.json'), Buffer.concat([
      Buffer.from([0xef, 0xbb, 0xbf]),
      Buffer.from(JSON.stringify(release, null, 2) + '\n'),
    ]));

    const finalFiles = (await listFiles(root)).filter((file) => file !== 'SHA256SUMS.txt');
    const finalLines = [];
    for (const file of finalFiles) {
      const bytes = await readFile(path.join(root, ...file.split('/')));
      finalLines.push(`${createHash('sha256').update(bytes).digest('hex')}  ${file}`);
    }
    await writeFile(path.join(root, 'SHA256SUMS.txt'), finalLines.join('\n') + '\n', 'ascii');

    const report = await auditWindowsPackage({
      packageRoot: root,
      commit,
      architecture: 'x64',
      releaseChannel: 'retail',
    });
    assert.equal(report.status, 'complete', report.issues.join('\n'));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
