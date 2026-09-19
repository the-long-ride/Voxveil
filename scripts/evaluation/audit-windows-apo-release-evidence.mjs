import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

const REQUIRED_SCENARIOS = [
  'capx-discovery',
  'real-processing',
  'effect-state-gating',
  'raw-mode',
  'default-endpoint-handoff',
  'graph-teardown',
  'install-uninstall-coexistence',
  'reboot-resume',
  'supported-hardware-matrix',
];

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function parseJsonBytes(bytes) {
  return JSON.parse(bytes.toString('utf8').replace(/^\uFEFF/, ''));
}

function validHash(value) {
  return typeof value === 'string' && /^[a-f0-9]{64}$/i.test(value);
}

function validTime(value) {
  return typeof value === 'string' && Number.isFinite(Date.parse(value));
}

function jsonFiles(directory, prefix) {
  if (!existsSync(directory)) return [];
  return readdirSync(directory)
    .filter((name) => name.startsWith(prefix) && name.endsWith('.json'))
    .map((name) => {
      const file = path.join(directory, name);
      try {
        return { name, file, data: parseJsonBytes(readFileSync(file)) };
      } catch (error) {
        return { name, file, error: String(error) };
      }
    });
}

function latest(items) {
  return [...items].sort(
    (left, right) => Date.parse(right.data?.observedAtUtc ?? 0) - Date.parse(left.data?.observedAtUtc ?? 0),
  )[0] ?? null;
}

function packageIdentity(args) {
  const root = path.resolve(args.packageRoot);
  const releasePath = path.join(root, 'release-manifest.json');
  const apoPath = path.join(root, 'system-audio', 'apo-verification.json');
  const issues = [];
  let release = null;
  let apo = null;
  let releaseBytes = null;
  let apoBytes = null;

  try {
    releaseBytes = readFileSync(releasePath);
    release = parseJsonBytes(releaseBytes);
  } catch (error) {
    issues.push(`release-manifest.json: missing or invalid (${error.message})`);
  }
  try {
    apoBytes = readFileSync(apoPath);
    apo = parseJsonBytes(apoBytes);
  } catch (error) {
    issues.push(`apo-verification.json: missing or invalid (${error.message})`);
  }

  if (release) {
    if (release.schemaVersion !== 1) issues.push('release-manifest.json: schemaVersion must be 1');
    if (release.voxveilCommit !== args.commit) issues.push('release-manifest.json: exact commit mismatch');
    if (release.architecture !== args.architecture) issues.push('release-manifest.json: architecture mismatch');
    if (release.signedApo?.present !== true) issues.push('release-manifest.json: signed APO is not present');
    if (release.signedVirtualDriver?.releaseChannel !== args.releaseChannel) {
      issues.push('release-manifest.json: release channel mismatch');
    }
  }

  const releaseManifestSha256 = releaseBytes ? sha256(releaseBytes) : null;
  const apoVerificationSha256 = apoBytes ? sha256(apoBytes) : null;
  if (release && apoBytes) {
    if (!validHash(release.signedApo?.verificationSha256) ||
        release.signedApo.verificationSha256 !== apoVerificationSha256) {
      issues.push('release-manifest.json: signed APO verification hash mismatch');
    }
  }
  if (apo) {
    if (apo.voxveilCommit !== args.commit) issues.push('apo-verification.json: exact commit mismatch');
    for (const field of [
      'apoInfSha256',
      'apoDllSha256',
      'apoCatalogSha256',
      'extensionInfSha256',
      'extensionCatalogSha256',
    ]) {
      if (!validHash(apo[field])) issues.push(`apo-verification.json: ${field} is invalid`);
    }
  }

  return {
    root,
    release,
    apo,
    releaseManifestSha256,
    apoVerificationSha256,
    issues,
  };
}

function validateEvidenceFiles(files, { required = false } = {}) {
  const issues = [];
  if (!Array.isArray(files)) return ['evidenceFiles must be an array'];
  if (required && files.length === 0) issues.push('at least one evidence artifact is required');
  for (const file of files) {
    if (typeof file?.fileName !== 'string' || !file.fileName ||
        !validHash(file?.sha256) ||
        !Number.isInteger(file?.bytes) ||
        file.bytes < (required ? 1 : 0)) {
      issues.push('evidenceFiles contains an invalid descriptor');
      break;
    }
  }
  return issues;
}

function validateScenario(item, identity, scenario) {
  if (!item) return [`missing APO validation scenario: ${scenario}`];
  if (item.error) return [`${item.name}: invalid JSON`];
  const x = item.data;
  const issues = [];
  if (x.schemaVersion !== 1) issues.push(`${scenario}: schemaVersion must be 1`);
  if (!validTime(x.observedAtUtc)) issues.push(`${scenario}: observedAtUtc is invalid`);
  if (x.releaseManifestSha256 !== identity.releaseManifestSha256) issues.push(`${scenario}: release manifest hash mismatch`);
  if (x.apoVerificationSha256 !== identity.apoVerificationSha256) issues.push(`${scenario}: APO verification hash mismatch`);
  if (x.voxveilCommit !== identity.release?.voxveilCommit) issues.push(`${scenario}: commit mismatch`);
  if (x.architecture !== identity.release?.architecture) issues.push(`${scenario}: architecture mismatch`);
  if (x.releaseChannel !== identity.release?.signedVirtualDriver?.releaseChannel) issues.push(`${scenario}: release channel mismatch`);
  if (x.scenario !== scenario) issues.push(`${scenario}: scenario identity mismatch`);
  if (x.result !== 'pass') issues.push(`${scenario}: latest result is ${x.result ?? 'missing'}, not pass`);
  if (typeof x.method !== 'string' || !x.method.trim()) issues.push(`${scenario}: method is missing`);
  if (typeof x.notes !== 'string' || !x.notes.trim()) issues.push(`${scenario}: notes are missing`);
  if (!Number.isInteger(x?.machine?.windowsBuild) || x.machine.windowsBuild < 22621) issues.push(`${scenario}: Windows build is below 22621`);
  if (x?.machine?.nativeArchitecture !== identity.release?.architecture) issues.push(`${scenario}: machine architecture mismatch`);
  if (x?.machine?.secureBootEnabled !== true) issues.push(`${scenario}: Secure Boot was not recorded enabled`);
  if (x?.machine?.testSigningEnabled !== false) issues.push(`${scenario}: TESTSIGNING was not recorded off`);
  if (typeof x?.machine?.bootMarker !== 'string' || !x.machine.bootMarker) issues.push(`${scenario}: boot marker is missing`);

  for (const field of [
    'apoInfSha256',
    'apoDllSha256',
    'apoCatalogSha256',
    'extensionInfSha256',
    'extensionCatalogSha256',
  ]) {
    if (x?.package?.[field] !== identity.apo?.[field]) issues.push(`${scenario}: package.${field} mismatch`);
  }
  issues.push(...validateEvidenceFiles(x.evidenceFiles, { required: scenario === 'supported-hardware-matrix' })
    .map((issue) => `${scenario}: ${issue}`));

  if (scenario === 'reboot-resume') {
    if (!validHash(x.priorEvidenceSha256)) issues.push('reboot-resume: prior evidence hash is invalid');
    if (typeof x.priorScenario !== 'string' || !x.priorScenario) issues.push('reboot-resume: prior scenario is missing');
    if (typeof x.priorBootMarker !== 'string' || !x.priorBootMarker) issues.push('reboot-resume: prior boot marker is missing');
    if (x.priorBootMarker === x?.machine?.bootMarker) issues.push('reboot-resume: boot marker did not change');
  }
  return issues;
}

function validateQualification(item, identity) {
  if (!item) return ['missing APO Retail qualification evidence'];
  if (item.error) return [`${item.name}: invalid JSON`];
  const x = item.data;
  const issues = [];
  if (x.schemaVersion !== 1) issues.push('qualification: schemaVersion must be 1');
  if (!validTime(x.observedAtUtc)) issues.push('qualification: observedAtUtc is invalid');
  if (x.releaseManifestSha256 !== identity.releaseManifestSha256) issues.push('qualification: release manifest hash mismatch');
  if (x.apoVerificationSha256 !== identity.apoVerificationSha256) issues.push('qualification: APO verification hash mismatch');
  if (x.voxveilCommit !== identity.release?.voxveilCommit) issues.push('qualification: commit mismatch');
  if (x.architecture !== identity.release?.architecture) issues.push('qualification: architecture mismatch');
  if (x.releaseChannel !== 'retail') issues.push('qualification: releaseChannel must be retail');
  if (!['whcp-hlk', 'microsoft-approved-retail'].includes(x.qualificationType)) {
    issues.push('qualification: qualificationType is invalid');
  }
  if (x.result !== 'pass') issues.push(`qualification: latest result is ${x.result ?? 'missing'}, not pass`);
  if (typeof x.method !== 'string' || !x.method.trim()) issues.push('qualification: method is missing');
  if (typeof x.notes !== 'string' || !x.notes.trim()) issues.push('qualification: notes are missing');
  issues.push(...validateEvidenceFiles(x.evidenceFiles, { required: true })
    .map((issue) => `qualification: ${issue}`));
  for (const field of [
    'apoInfSha256',
    'apoDllSha256',
    'apoCatalogSha256',
    'extensionInfSha256',
    'extensionCatalogSha256',
  ]) {
    if (x?.package?.[field] !== identity.apo?.[field]) issues.push(`qualification: package.${field} mismatch`);
  }
  return issues;
}

export function auditWindowsApoEvidence(args) {
  const identity = packageIdentity(args);
  const issues = [...identity.issues];
  const measurements = path.resolve(args.workspace, 'measurements');

  const validation = jsonFiles(measurements, 'windows-apo-validation-')
    .filter((item) => !item.error &&
      item.data?.releaseManifestSha256 === identity.releaseManifestSha256 &&
      item.data?.apoVerificationSha256 === identity.apoVerificationSha256);
  const qualifications = jsonFiles(measurements, 'windows-apo-qualification-')
    .filter((item) => !item.error &&
      item.data?.releaseManifestSha256 === identity.releaseManifestSha256 &&
      item.data?.apoVerificationSha256 === identity.apoVerificationSha256);

  const scenarioRecords = {};
  for (const scenario of REQUIRED_SCENARIOS) {
    const item = latest(validation.filter((candidate) => candidate.data?.scenario === scenario));
    scenarioRecords[scenario] = item?.name ?? null;
    issues.push(...validateScenario(item, identity, scenario));
  }

  let qualification = null;
  if (args.releaseChannel === 'retail') {
    const item = latest(qualifications);
    qualification = item?.name ?? null;
    issues.push(...validateQualification(item, identity));
  }

  return {
    status: issues.length === 0 ? 'complete' : 'incomplete',
    commit: args.commit,
    architecture: args.architecture,
    releaseChannel: args.releaseChannel,
    packageRoot: identity.root,
    releaseManifestSha256: identity.releaseManifestSha256,
    apoVerificationSha256: identity.apoVerificationSha256,
    scenarioRecords,
    qualification,
    issues,
  };
}
