import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { auditWindowsApoEvidence } from '../evaluation/audit-windows-apo-release-evidence.mjs';

const commit = 'a'.repeat(40);
const H = (value) => createHash('sha256').update(value).digest('hex');

function writeJson(file, value) {
  writeFileSync(file, JSON.stringify(value, null, 2) + '\n');
}

function packageFixture(root) {
  const systemAudio = path.join(root, 'system-audio');
  mkdirSync(systemAudio, { recursive: true });

  const apo = {
    voxveilCommit: commit,
    apoInfSha256: '1'.repeat(64),
    apoDllSha256: '2'.repeat(64),
    apoCatalogSha256: '3'.repeat(64),
    extensionInfSha256: '4'.repeat(64),
    extensionCatalogSha256: '5'.repeat(64),
  };
  const apoPath = path.join(systemAudio, 'apo-verification.json');
  writeJson(apoPath, apo);
  const apoHash = createHash('sha256').update(readFileSync(apoPath)).digest('hex');

  const release = {
    schemaVersion: 1,
    voxveilCommit: commit,
    architecture: 'x64',
    signedApo: {
      present: true,
      verificationSha256: apoHash,
    },
    signedVirtualDriver: {
      present: true,
      releaseChannel: 'retail',
      verificationSha256: '6'.repeat(64),
    },
    packageFilesHashedBy: 'SHA256SUMS.txt',
  };
  const releasePath = path.join(root, 'release-manifest.json');
  writeJson(releasePath, release);

  return {
    releaseHash: createHash('sha256').update(readFileSync(releasePath)).digest('hex'),
    apoHash,
    apo,
  };
}

function validationRecord(identity, scenario, time, overrides = {}) {
  return {
    schemaVersion: 1,
    observedAtUtc: time,
    releaseManifestSha256: identity.releaseHash,
    apoVerificationSha256: identity.apoHash,
    voxveilCommit: commit,
    architecture: 'x64',
    releaseChannel: 'retail',
    package: {
      apoInfSha256: identity.apo.apoInfSha256,
      apoDllSha256: identity.apo.apoDllSha256,
      apoCatalogSha256: identity.apo.apoCatalogSha256,
      extensionInfSha256: identity.apo.extensionInfSha256,
      extensionCatalogSha256: identity.apo.extensionCatalogSha256,
    },
    machine: {
      windowsVersion: '10.0.26100',
      windowsBuild: 26100,
      nativeArchitecture: 'x64',
      secureBootEnabled: true,
      testSigningEnabled: false,
      bootMarker: '2026-09-18T00:00:00.000Z',
    },
    scenario,
    result: 'pass',
    method: 'real-machine observation',
    notes: 'Observed expected production APO behavior.',
    priorEvidenceSha256: null,
    priorScenario: null,
    priorBootMarker: null,
    evidenceFiles: scenario === 'supported-hardware-matrix'
      ? [{ fileName: 'hardware-matrix.json', sha256: H('matrix'), bytes: 100 }]
      : [],
    ...overrides,
  };
}

test('APO release evidence audit accepts a complete Retail matrix and fails closed on a latest blocked scenario', () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'voxveil-apo-evidence-'));
  const packageRoot = path.join(root, 'package');
  const workspace = path.join(root, 'workspace');
  const measurements = path.join(workspace, 'measurements');
  mkdirSync(packageRoot, { recursive: true });
  mkdirSync(measurements, { recursive: true });

  try {
    const identity = packageFixture(packageRoot);
    const scenarios = [
      'capx-discovery',
      'real-processing',
      'effect-state-gating',
      'raw-mode',
      'default-endpoint-handoff',
      'graph-teardown',
      'install-uninstall-coexistence',
      'supported-hardware-matrix',
    ];

    let index = 1;
    let priorPath = null;
    for (const scenario of scenarios) {
      const file = path.join(measurements, `windows-apo-validation-${scenario}-${index}.json`);
      writeJson(file, validationRecord(identity, scenario, `2026-09-19T00:${String(index).padStart(2, '0')}:00.000Z`));
      if (scenario === 'real-processing') priorPath = file;
      index += 1;
    }

    const prior = JSON.parse(readFileSync(priorPath, 'utf8'));
    const priorHash = createHash('sha256').update(readFileSync(priorPath)).digest('hex');
    writeJson(
      path.join(measurements, 'windows-apo-validation-reboot-resume-9.json'),
      validationRecord(identity, 'reboot-resume', '2026-09-19T00:09:00.000Z', {
        machine: {
          ...prior.machine,
          bootMarker: '2026-09-19T00:08:00.000Z',
        },
        priorEvidenceSha256: priorHash,
        priorScenario: prior.scenario,
        priorBootMarker: prior.machine.bootMarker,
      }),
    );

    writeJson(path.join(measurements, 'windows-apo-qualification-whcp-hlk.json'), {
      schemaVersion: 1,
      observedAtUtc: '2026-09-19T00:10:00.000Z',
      releaseManifestSha256: identity.releaseHash,
      apoVerificationSha256: identity.apoHash,
      voxveilCommit: commit,
      architecture: 'x64',
      releaseChannel: 'retail',
      package: {
        apoInfSha256: identity.apo.apoInfSha256,
        apoDllSha256: identity.apo.apoDllSha256,
        apoCatalogSha256: identity.apo.apoCatalogSha256,
        extensionInfSha256: identity.apo.extensionInfSha256,
        extensionCatalogSha256: identity.apo.extensionCatalogSha256,
      },
      qualificationType: 'whcp-hlk',
      result: 'pass',
      method: 'HLK package review',
      notes: 'Applicable APO/audio tests completed and evidence retained externally.',
      evidenceFiles: [{ fileName: 'apo-results.hlkx', sha256: H('hlkx'), bytes: 1234 }],
    });

    const ok = auditWindowsApoEvidence({
      workspace,
      packageRoot,
      commit,
      architecture: 'x64',
      releaseChannel: 'retail',
    });
    assert.equal(ok.status, 'complete', ok.issues.join('\n'));

    writeJson(
      path.join(measurements, 'windows-apo-validation-default-endpoint-handoff-latest.json'),
      validationRecord(identity, 'default-endpoint-handoff', '2026-09-19T01:00:00.000Z', {
        result: 'blocked',
        notes: 'Blocked pending endpoint-switch observation.',
      }),
    );

    const blocked = auditWindowsApoEvidence({
      workspace,
      packageRoot,
      commit,
      architecture: 'x64',
      releaseChannel: 'retail',
    });
    assert.equal(blocked.status, 'incomplete');
    assert.ok(blocked.issues.some((issue) => issue.includes('default-endpoint-handoff') && issue.includes('blocked')));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('APO release evidence audit rejects a reboot record whose prior hash is not a bound record', () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'voxveil-apo-reboot-'));
  const packageRoot = path.join(root, 'package');
  const workspace = path.join(root, 'workspace');
  const measurements = path.join(workspace, 'measurements');
  mkdirSync(packageRoot, { recursive: true });
  mkdirSync(measurements, { recursive: true });

  try {
    const identity = packageFixture(packageRoot);
    writeJson(
      path.join(measurements, 'windows-apo-validation-reboot-resume.json'),
      validationRecord(identity, 'reboot-resume', '2026-09-19T00:09:00.000Z', {
        machine: {
          windowsVersion: '10.0.26100',
          windowsBuild: 26100,
          nativeArchitecture: 'x64',
          secureBootEnabled: true,
          testSigningEnabled: false,
          bootMarker: '2026-09-19T00:08:00.000Z',
        },
        priorEvidenceSha256: 'f'.repeat(64),
        priorScenario: 'real-processing',
        priorBootMarker: '2026-09-18T00:00:00.000Z',
      }),
    );

    const report = auditWindowsApoEvidence({
      workspace,
      packageRoot,
      commit,
      architecture: 'x64',
      releaseChannel: 'retail',
    });
    assert.equal(report.status, 'incomplete');
    assert.ok(report.issues.some((issue) => /prior evidence hash does not identify/i.test(issue)));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
