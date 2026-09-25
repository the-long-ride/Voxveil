import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const auditPath = 'scripts/evaluation/audit-windows-driver-release-evidence.mjs';
const lifecycleRecorder = readFileSync('scripts/evaluation/record-windows-driver-lifecycle-evidence.ps1', 'utf8');
const qualificationRecorder = readFileSync('scripts/evaluation/record-windows-driver-qualification-evidence.ps1', 'utf8');

const H = (c) => c.repeat(64);
const commit = 'a'.repeat(40);

function hash(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function writeJson(path, value) {
  writeFileSync(path, JSON.stringify(value, null, 2));
}

function runAudit(workspace, releaseChannel = 'retail') {
  return spawnSync(process.execPath, [
    auditPath,
    '--workspace', workspace,
    '--commit', commit,
    '--architecture', 'x64',
    '--release-channel', releaseChannel,
    '--json',
  ], { encoding: 'utf8' });
}

test('Windows driver release evidence audit accepts a complete Retail matrix and fails closed when a latest scenario is blocked', () => {
  const workspace = mkdtempSync(join(tmpdir(), 'voxveil-driver-evidence-'));
  const measurements = join(workspace, 'measurements');
  mkdirSync(measurements, { recursive: true });

  try {
    const prePath = join(measurements, 'signed-driver-preinstall-x64-abc-20260919T000000Z.json');
    const pre = {
      generatedAtUtc: '2026-09-19T00:00:00.000Z',
      voxveilCommit: commit,
      releaseChannel: 'retail',
      architecture: 'x64',
      package: {
        infSha256: H('1'),
        catalogSha256: H('2'),
        driverSha256: H('3'),
        catalogSigner: 'CN=Microsoft Windows Hardware Compatibility Publisher',
        catalogThumbprint: '4'.repeat(40),
        releaseEvidenceSha256: H('5'),
        submissionManifestSha256: H('6'),
        unsignedCatalogSha256: H('7'),
        unsignedPdbSha256: H('8'),
        signingPath: 'whcp-hlk',
      },
      machine: {
        windowsVersion: '10.0.26100',
        windowsBuild: 26100,
        nativeArchitecture: 'x64',
        secureBoot: { enabled: true },
        testSigning: { enabled: false },
      },
      preInstallStatus: 'verified',
      lifecycle: { status: 'pending' },
    };
    writeJson(prePath, pre);
    const preHash = hash(prePath);

    const scenarios = [
      'clean-install',
      'reboot-resume',
      'same-package-repair',
      'uninstall',
      'reinstall',
      'package-replacement',
    ];
    scenarios.forEach((scenario, index) => {
      writeJson(join(measurements, `signed-driver-lifecycle-${scenario}-a-${index}.json`), {
        schemaVersion: 1,
        observedAtUtc: `2026-09-19T00:0${index + 1}:00.000Z`,
        preInstallEvidenceSha256: preHash,
        voxveilCommit: commit,
        releaseChannel: 'retail',
        architecture: 'x64',
        package: {
          infSha256: H('1'),
          catalogSha256: H('2'),
          driverSha256: H('3'),
          submissionManifestSha256: H('6'),
        },
        machine: {
          windowsVersion: '10.0.26100',
          windowsBuild: 26100,
          nativeArchitecture: 'x64',
          secureBootEnabled: true,
          testSigningEnabled: false,
          bootMarker: scenario === 'reboot-resume' ? '2026-09-19T00:00:01.000Z' : '2026-09-18T00:00:00.000Z',
        },
        scenario,
        result: 'pass',
        method: 'real-machine observation',
        notes: 'Observed expected behavior.',
        priorLifecycleEvidenceSha256: scenario === 'reboot-resume' ? H('9') : null,
        priorScenario: scenario === 'reboot-resume' ? 'clean-install' : null,
        priorBootMarker: scenario === 'reboot-resume' ? '2026-09-18T00:00:00.000Z' : null,
        evidenceFiles: [],
      });
    });

    writeJson(join(measurements, 'signed-driver-qualification-whcp-hlk-a.json'), {
      schemaVersion: 1,
      observedAtUtc: '2026-09-19T00:10:00.000Z',
      preInstallEvidenceSha256: preHash,
      voxveilCommit: commit,
      releaseChannel: 'retail',
      architecture: 'x64',
      package: {
        infSha256: H('1'),
        catalogSha256: H('2'),
        driverSha256: H('3'),
        submissionManifestSha256: H('6'),
        signingPath: 'whcp-hlk',
      },
      qualificationType: 'whcp-hlk',
      result: 'pass',
      method: 'HLK package review',
      notes: 'Required playlist completed and retained externally.',
      evidenceFiles: [{ fileName: 'Voxveil.hlkx', sha256: H('b'), bytes: 1234 }],
    });

    const ok = runAudit(workspace);
    assert.equal(ok.status, 0, ok.stderr || ok.stdout);
    const okJson = JSON.parse(ok.stdout);
    assert.equal(okJson.status, 'complete');

    writeJson(join(measurements, 'signed-driver-lifecycle-package-replacement-a-latest.json'), {
      schemaVersion: 1,
      observedAtUtc: '2026-09-19T01:00:00.000Z',
      preInstallEvidenceSha256: preHash,
      voxveilCommit: commit,
      releaseChannel: 'retail',
      architecture: 'x64',
      package: {
        infSha256: H('1'),
        catalogSha256: H('2'),
        driverSha256: H('3'),
        submissionManifestSha256: H('6'),
      },
      machine: {
        windowsVersion: '10.0.26100',
        windowsBuild: 26100,
        nativeArchitecture: 'x64',
        secureBootEnabled: true,
        testSigningEnabled: false,
        bootMarker: '2026-09-18T00:00:00.000Z',
      },
      scenario: 'package-replacement',
      result: 'blocked',
      method: 'real-machine observation',
      notes: 'Blocked pending replacement package.',
      evidenceFiles: [],
    });

    const blocked = runAudit(workspace);
    assert.equal(blocked.status, 1);
    const blockedJson = JSON.parse(blocked.stdout);
    assert.equal(blockedJson.status, 'incomplete');
    assert.ok(blockedJson.issues.some((issue) => issue.includes('package-replacement') && issue.includes('blocked')));
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('lifecycle recorder binds observations to verified machine state and proves reboot-resume with a changed boot marker', () => {
  assert.match(lifecycleRecorder, /preInstallEvidenceSha256/i);
  assert.match(lifecycleRecorder, /Confirm-SecureBootUEFI/i);
  assert.match(lifecycleRecorder, /TESTSIGNING must remain off/i);
  assert.match(lifecycleRecorder, /LastBootUpTime/i);
  assert.match(lifecycleRecorder, /PriorLifecycleEvidence/i);
  assert.match(lifecycleRecorder, /changed Windows boot marker/i);
  assert.match(lifecycleRecorder, /evidenceFiles/i);
  assert.doesNotMatch(lifecycleRecorder, /git\s+(?:add|commit|push)/i);
});

test('Retail qualification recorder hashes external evidence without copying paths or credentials into the record', () => {
  assert.match(qualificationRecorder, /whcp-hlk/i);
  assert.match(qualificationRecorder, /microsoft-approved-retail/i);
  assert.match(qualificationRecorder, /Get-Sha256/i);
  assert.match(qualificationRecorder, /fileName/i);
  assert.match(qualificationRecorder, /bytes/i);
  assert.match(qualificationRecorder, /Only evidence artifact file names, sizes, and SHA-256/i);
  assert.doesNotMatch(qualificationRecorder, /Partner Center credentials\s*=/i);
  assert.doesNotMatch(qualificationRecorder, /Copy-Item\s+\$.*Evidence/i);
});
