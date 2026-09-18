import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile, unlink } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { auditWorkspace, manualCoverageChecks } from '../evaluation/audit-classic-dsp-evidence.mjs';

const HASH = 'a'.repeat(64);

async function writeJson(file, value) {
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, JSON.stringify(value, null, 2));
}

function manifest(fixtureId, tier, rate, status = tier === 'controlled' ? 'prepared' : 'approved-metadata') {
  return {
    fixtureId,
    tier,
    status,
    targetSampleRate: rate,
    durationSeconds: 20,
    sources: tier === 'controlled'
      ? [
          {
            role: 'vocal',
            dataset: 'VocalSet',
            sourceRecord: 'vocal-record',
            sourceFile: 'sources/vocal.wav',
            license: 'CC-BY-4.0',
            licenseCheckedOn: '2026-09-19',
            sourceSha256: HASH,
          },
          {
            role: 'accompaniment',
            dataset: 'URMP',
            sourceRecord: 'urmp-record',
            sourceFile: 'sources/music.wav',
            license: 'CC0-1.0',
            licenseCheckedOn: '2026-09-19',
            sourceSha256: HASH,
          },
        ]
      : [
          {
            role: 'mixed',
            sourceRecord: 'natural-record',
            sourceFile: 'sources/natural.ogg',
            license: 'CC-BY-4.0',
            licenseCheckedOn: '2026-09-19',
            sourceSha256: HASH,
          },
        ],
    mixRecipe: tier === 'controlled'
      ? {
          vocalStartSeconds: 0,
          accompanimentStartSeconds: 0,
          vocalGainDb: -6,
          accompanimentGainDb: 0,
          vocalPan: 'center',
          normalization: 'none',
          preparationCommand: 'fixture-builder',
          preparationToolVersion: 'ffmpeg test',
        }
      : null,
    fixtureSha256: HASH,
    notes: '',
  };
}

function renderEvidence(fixtureId, tier, rate) {
  return {
    fixtureId,
    tier,
    sampleRate: rate,
    vocal: 0,
    source: { nativeSampleRate: rate, sha256: HASH },
    input: { sha256: HASH },
    renders: {
      musicPreservation: { sha256: HASH },
      balanced: { sha256: HASH },
    },
    frameCountParity: true,
    objectiveStatus: 'rendered',
    subjectiveReview: {
      status: 'complete',
      decision: 'accepted',
      musicPreservation: 'accepted listening notes',
      balanced: 'accepted listening notes',
      reviewMethod: 'randomized A/B',
    },
  };
}

function runtimeEvidence(rate, profile, workload) {
  return {
    machine: {
      windowsBuild: '26100',
      osArchitecture: '64-bit',
      logicalProcessorCount: 16,
      cpuModels: ['Test CPU'],
    },
    route: {
      sourceEndpoint: 'Voxveil Input',
      physicalOutput: 'Test Speakers',
      sampleRate: rate,
      profile,
      vocal: 0,
    },
    cpu: {
      status: 'sampled',
      workloadState: workload,
      normalizedPercent: workload === 'idle' ? 0.5 : 4.5,
    },
    manualMeasurements: workload === 'processing'
      ? {
          status: 'complete',
          dropoutCount: 0,
          endToEndLatencyMs: 23.5,
          latencyMethod: 'loopback impulse',
        }
      : {
          status: 'pending',
          dropoutCount: null,
          endToEndLatencyMs: null,
          latencyMethod: '',
        },
  };
}

async function createCompleteWorkspace(root) {
  const manifests = path.join(root, 'manifests');
  const measurements = path.join(root, 'measurements');
  const fixtures = [
    ['controlled-44-a', 'controlled', 44100],
    ['controlled-44-b', 'controlled', 44100],
    ['controlled-48-a', 'controlled', 48000],
    ['controlled-48-b', 'controlled', 48000],
    ['natural-44-a', 'natural-mix', 44100],
  ];

  for (const [id, tier, rate] of fixtures) {
    await writeJson(path.join(manifests, `${id}.json`), manifest(id, tier, rate));
    await writeJson(path.join(measurements, `${id}-${rate}-render-evidence.json`), renderEvidence(id, tier, rate));
  }

  for (const rate of [44100, 48000]) {
    for (const profile of ['music-preservation', 'balanced']) {
      for (const workload of ['idle', 'processing']) {
        await writeJson(
          path.join(measurements, `windows-runtime-${rate}-${profile}-${workload}-test.json`),
          runtimeEvidence(rate, profile, workload),
        );
      }
    }
  }
}

test('evidence audit accepts a structurally complete matrix without claiming semantic coverage', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-evidence-'));
  try {
    await createCompleteWorkspace(root);
    const report = await auditWorkspace(root);
    assert.equal(report.structuralComplete, true);
    assert.deepEqual(report.issues, []);
    assert.equal(report.summary.controlledAccepted44100, 2);
    assert.equal(report.summary.controlledAccepted48000, 2);
    assert.equal(report.summary.naturalMixAccepted, 1);
    assert.equal(report.summary.runtimeMatrixComplete, true);
    assert.ok(manualCoverageChecks.some((item) => /male and female/i.test(item)));
    assert.ok(manualCoverageChecks.some((item) => /sparse and dense/i.test(item)));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('evidence audit fails closed when one runtime processing record is missing', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-evidence-'));
  try {
    await createCompleteWorkspace(root);
    await unlink(path.join(root, 'measurements', 'windows-runtime-48000-balanced-processing-test.json'));
    const report = await auditWorkspace(root);
    assert.equal(report.structuralComplete, false);
    assert.equal(report.summary.runtimeMatrixComplete, false);
    assert.ok(report.issues.some((issue) => /48000\/balanced\/processing/i.test(issue)));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
