import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { auditReleaseReadiness } from '../evaluation/audit-release-readiness.mjs';

const commit = 'a'.repeat(40);
const args = {
  commit,
  architecture: 'x64',
  releaseChannel: 'retail',
  classicWorkspace: '.local-evaluation/classic-dsp',
  driverWorkspace: '.local-evaluation/windows-driver',
  apoWorkspace: '.local-evaluation/windows-apo',
  packageRoot: 'dist/windows-x64/Voxveil',
};

test('release-readiness audit requires package, Classic DSP, and Windows driver evidence for the exact checkout', async () => {
  const report = await auditReleaseReadiness(args, {
    checkoutCommit: commit,
    packageAudit: async () => ({
      status: 'complete',
      signedApoPresent: true,
      issues: [],
    }),
    apoAudit: () => ({
      status: 'complete',
      issues: [],
    }),
    classicAudit: async () => ({
      structuralComplete: true,
      issues: [],
      summary: { semanticCoverageComplete: true, runtimeMatrixComplete: true },
    }),
    driverAudit: () => ({
      status: 'complete',
      issues: [],
    }),
  });

  assert.equal(report.status, 'complete');
  assert.deepEqual(report.issues, []);
  assert.equal(report.checkoutCommit, commit);
  assert.equal(report.windowsPackage.status, 'complete');
  assert.equal(report.windowsApo.status, 'complete');
  assert.equal(report.classicDsp.structuralComplete, true);
  assert.equal(report.windowsDriver.status, 'complete');
});

test('release-readiness audit fails closed on checkout mismatch before accepting evidence', async () => {
  let packageCalled = false;
  let apoCalled = false;
  let classicCalled = false;
  let driverCalled = false;
  const report = await auditReleaseReadiness(args, {
    checkoutCommit: 'b'.repeat(40),
    packageAudit: async () => {
      packageCalled = true;
      return { status: 'complete', issues: [] };
    },
    apoAudit: () => {
      apoCalled = true;
      return { status: 'complete', issues: [] };
    },
    classicAudit: async () => {
      classicCalled = true;
      return { structuralComplete: true, issues: [] };
    },
    driverAudit: () => {
      driverCalled = true;
      return { status: 'complete', issues: [] };
    },
  });

  assert.equal(report.status, 'incomplete');
  assert.equal(packageCalled, false);
  assert.equal(apoCalled, false);
  assert.equal(classicCalled, false);
  assert.equal(driverCalled, false);
  assert.ok(report.issues.some((issue) => /does not match current checkout/i.test(issue)));
});

test('release-readiness audit prefixes failures from every evidence domain', async () => {
  const report = await auditReleaseReadiness(args, {
    checkoutCommit: commit,
    packageAudit: async () => ({
      status: 'incomplete',
      signedApoPresent: true,
      issues: ['package checksum mismatch'],
    }),
    apoAudit: () => ({
      status: 'incomplete',
      issues: ['default-endpoint-handoff: latest result is blocked, not pass'],
    }),
    classicAudit: async () => ({
      structuralComplete: false,
      issues: ['semantic coverage review is missing'],
    }),
    driverAudit: () => ({
      status: 'incomplete',
      issues: ['missing Retail qualification evidence'],
    }),
  });

  assert.equal(report.status, 'incomplete');
  assert.ok(report.issues.includes('windows-package: package checksum mismatch'));
  assert.ok(report.issues.includes('windows-apo: default-endpoint-handoff: latest result is blocked, not pass'));
  assert.ok(report.issues.includes('classic-dsp: semantic coverage review is missing'));
  assert.ok(report.issues.includes('windows-driver: missing Retail qualification evidence'));
});

test('release-readiness CLI binds the real audits and Git HEAD instead of shelling out to separate audit commands', () => {
  const source = readFileSync('scripts/evaluation/audit-release-readiness.mjs', 'utf8');
  assert.match(source, /auditWorkspace as auditClassicDspEvidence/i);
  assert.match(source, /auditWindowsApoEvidence/i);
  assert.match(source, /auditWindowsDriverEvidence/i);
  assert.match(source, /auditWindowsPackage/i);
  assert.match(source, /execFileSync\('git', \['rev-parse', 'HEAD'\]/i);
  assert.match(source, /requested commit .* does not match current checkout/i);
  assert.match(source, /--apo-workspace/i);
  assert.match(source, /--package-root/i);
  assert.doesNotMatch(source, /npm run evaluation:audit(?:-windows-driver)?/i);
});

test('release-readiness audit skips APO evidence only when the final package omits the signed APO', async () => {
  let apoCalled = false;
  const report = await auditReleaseReadiness(args, {
    checkoutCommit: commit,
    packageAudit: async () => ({
      status: 'complete',
      signedApoPresent: false,
      issues: [],
    }),
    apoAudit: () => {
      apoCalled = true;
      return { status: 'complete', issues: [] };
    },
    classicAudit: async () => ({ structuralComplete: true, issues: [] }),
    driverAudit: () => ({ status: 'complete', issues: [] }),
  });

  assert.equal(report.status, 'complete');
  assert.equal(apoCalled, false);
  assert.equal(report.windowsApo.status, 'not-applicable');
});
