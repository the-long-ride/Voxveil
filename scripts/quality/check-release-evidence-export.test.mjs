import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { buildReleaseEvidenceManifest } from '../evaluation/export-release-evidence-manifest.mjs';

const commit = 'a'.repeat(40);

async function fixture() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-release-evidence-'));
  const classic = path.join(root, 'classic');
  const driver = path.join(root, 'driver');
  const apo = path.join(root, 'apo');
  const packageRoot = path.join(root, 'package');
  await mkdir(path.join(classic, 'measurements'), { recursive: true });
  await mkdir(path.join(driver, 'measurements'), { recursive: true });
  await mkdir(path.join(apo, 'measurements'), { recursive: true });
  await mkdir(packageRoot, { recursive: true });

  await writeFile(path.join(classic, 'measurements', 'coverage.json'), '{"kind":"classic"}\n');
  await writeFile(path.join(driver, 'measurements', 'lifecycle.json'), '{"kind":"driver"}\n');
  await writeFile(path.join(apo, 'measurements', 'capx.json'), '{"kind":"apo"}\n');
  await writeFile(path.join(packageRoot, 'release-manifest.json'), '{"schemaVersion":1}\n');
  await writeFile(path.join(packageRoot, 'SHA256SUMS.txt'), '0'.repeat(64) + '  release-manifest.json\n');

  return {
    root,
    args: {
      commit,
      architecture: 'x64',
      releaseChannel: 'retail',
      classicWorkspace: classic,
      driverWorkspace: driver,
      apoWorkspace: apo,
      packageRoot,
    },
  };
}

function completeAudit() {
  return {
    status: 'complete',
    issues: [],
    windowsPackage: {
      status: 'complete',
      signedApoPresent: true,
      signedVirtualDriverPresent: true,
      filesChecked: 42,
    },
    windowsApo: {
      status: 'complete',
      scenarioRecords: { 'real-processing': 'windows-apo-validation-real-processing.json' },
      qualification: 'windows-apo-qualification-whcp-hlk.json',
    },
    classicDsp: {
      structuralComplete: true,
      summary: {
        coverageReviewFile: '/private/path/classic-dsp-coverage-review.json',
      },
    },
    windowsDriver: {
      status: 'complete',
      preInstallEvidence: 'preinstall.json',
      scenarioRecords: { 'clean-install': 'clean-install.json' },
      qualification: 'driver-qualification.json',
    },
  };
}

test('release evidence export refuses an incomplete unified audit', async () => {
  const { root, args } = await fixture();
  try {
    await assert.rejects(
      buildReleaseEvidenceManifest(args, {
        releaseAudit: async () => ({ status: 'incomplete', issues: ['missing hardware evidence'] }),
      }),
      /Refusing to export release evidence.*missing hardware evidence/i,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('release evidence manifest inventories evidence without leaking absolute workspace paths', async () => {
  const { root, args } = await fixture();
  try {
    const manifest = await buildReleaseEvidenceManifest(args, {
      releaseAudit: async () => completeAudit(),
      now: () => new Date('2026-09-19T10:00:00.000Z'),
    });

    assert.equal(manifest.schemaVersion, 1);
    assert.equal(manifest.voxveilCommit, commit);
    assert.equal(manifest.auditStatus.windowsPackage, 'complete');
    assert.equal(manifest.auditStatus.windowsApo, 'complete');
    assert.equal(manifest.auditStatus.classicDsp, 'complete');
    assert.equal(manifest.auditStatus.windowsDriver, 'complete');
    assert.equal(manifest.evidenceFiles.length, 3);
    assert.deepEqual(
      manifest.evidenceFiles.map((item) => item.domain),
      ['classic-dsp', 'windows-apo', 'windows-driver'],
    );
    assert.equal(manifest.selectedEvidence.classicDspCoverageReview, 'classic-dsp-coverage-review.json');
    assert.match(manifest.evidenceSetSha256, /^[a-f0-9]{64}$/);

    const serialized = JSON.stringify(manifest);
    assert.equal(serialized.includes(root), false);
    for (const item of manifest.evidenceFiles) {
      assert.match(item.sha256, /^[a-f0-9]{64}$/);
      assert.ok(item.bytes > 0);
      assert.equal(path.isAbsolute(item.relativePath), false);
    }

    const releaseBytes = await readFile(path.join(args.packageRoot, 'release-manifest.json'));
    const expectedReleaseHash = createHash('sha256').update(releaseBytes).digest('hex');
    assert.equal(
      manifest.package.criticalFiles.find((item) => item.relativePath === 'release-manifest.json').sha256,
      expectedReleaseHash,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('evidence-set hash is stable across export timestamps for unchanged evidence', async () => {
  const { root, args } = await fixture();
  try {
    const first = await buildReleaseEvidenceManifest(args, {
      releaseAudit: async () => completeAudit(),
      now: () => new Date('2026-09-19T10:00:00.000Z'),
    });
    const second = await buildReleaseEvidenceManifest(args, {
      releaseAudit: async () => completeAudit(),
      now: () => new Date('2026-09-20T10:00:00.000Z'),
    });
    assert.notEqual(first.createdAtUtc, second.createdAtUtc);
    assert.equal(first.evidenceSetSha256, second.evidenceSetSha256);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
