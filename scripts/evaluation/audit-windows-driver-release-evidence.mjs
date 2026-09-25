import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { resolve, join } from 'node:path';

const REQUIRED_SCENARIOS = [
  'clean-install',
  'reboot-resume',
  'same-package-repair',
  'uninstall',
  'reinstall',
  'package-replacement',
];

function parseArgs(argv) {
  const args = {
    workspace: '.local-evaluation/windows-driver',
    architecture: 'x64',
    releaseChannel: 'pilot',
    json: false,
    commit: null,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (token === '--json') {
      args.json = true;
      continue;
    }
    const next = argv[i + 1];
    if (!next) throw new Error(`Missing value for ${token}`);
    if (token === '--workspace') args.workspace = next;
    else if (token === '--commit') args.commit = next.toLowerCase();
    else if (token === '--architecture') args.architecture = next;
    else if (token === '--release-channel') args.releaseChannel = next.toLowerCase();
    else throw new Error(`Unknown argument: ${token}`);
    i += 1;
  }
  if (!args.commit || !/^[a-f0-9]{40}$/.test(args.commit)) {
    throw new Error('--commit must be the exact 40-hex Voxveil commit.');
  }
  if (!['x64', 'ARM64'].includes(args.architecture)) {
    throw new Error('--architecture must be x64 or ARM64.');
  }
  if (!['pilot', 'retail'].includes(args.releaseChannel)) {
    throw new Error('--release-channel must be pilot or retail.');
  }
  return args;
}

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function jsonFiles(directory, prefix) {
  if (!existsSync(directory)) return [];
  return readdirSync(directory)
    .filter((name) => name.startsWith(prefix) && name.endsWith('.json'))
    .map((name) => {
      const path = join(directory, name);
      try {
        return { name, path, data: JSON.parse(readFileSync(path, 'utf8')) };
      } catch (error) {
        return { name, path, error: String(error) };
      }
    });
}

function validHash(value, length = 64) {
  return typeof value === 'string' && new RegExp(`^[a-f0-9]{${length}}$`, 'i').test(value);
}

function validTime(value) {
  return typeof value === 'string' && Number.isFinite(Date.parse(value));
}

function validatePreInstall(item, args) {
  const issues = [];
  if (item.error) return [`${item.name}: invalid JSON`];
  const x = item.data;
  if (x.voxveilCommit !== args.commit) issues.push('commit mismatch');
  if (x.architecture !== args.architecture) issues.push('architecture mismatch');
  if (x.releaseChannel !== args.releaseChannel) issues.push('release-channel mismatch');
  if (x.preInstallStatus !== 'verified') issues.push('preInstallStatus is not verified');
  if (!validTime(x.generatedAtUtc)) issues.push('generatedAtUtc is invalid');
  if (!Number.isInteger(x?.machine?.windowsBuild) || x.machine.windowsBuild < 22621) issues.push('Windows build is below 22621');
  if (x?.machine?.nativeArchitecture !== args.architecture) issues.push('native architecture mismatch');
  if (x?.machine?.secureBoot?.enabled !== true) issues.push('Secure Boot is not proven enabled');
  if (x?.machine?.testSigning?.enabled !== false) issues.push('TESTSIGNING is not proven off');
  for (const field of [
    'infSha256',
    'catalogSha256',
    'driverSha256',
    'submissionManifestSha256',
    'unsignedCatalogSha256',
    'unsignedPdbSha256',
  ]) {
    if (!validHash(x?.package?.[field])) issues.push(`package.${field} is invalid`);
  }
  if (!validHash(x?.package?.catalogThumbprint, 40)) issues.push('catalog thumbprint is invalid');
  if (typeof x?.package?.catalogSigner !== 'string' || !x.package.catalogSigner.trim()) issues.push('catalog signer is missing');
  if (args.releaseChannel === 'retail') {
    if (!['whcp-hlk', 'microsoft-approved-retail'].includes(x?.package?.signingPath)) {
      issues.push('Retail signingPath is not approved');
    }
    if (!validHash(x?.package?.releaseEvidenceSha256)) issues.push('Retail releaseEvidenceSha256 is invalid');
  } else if (x?.package?.signingPath !== 'attestation-pilot') {
    issues.push('Pilot signingPath must be attestation-pilot');
  }
  return issues;
}

function latestByTime(items, field = 'observedAtUtc') {
  return [...items].sort((a, b) => Date.parse(b.data?.[field] ?? 0) - Date.parse(a.data?.[field] ?? 0))[0] ?? null;
}

function validateLifecycle(item, pre, preHash, scenario) {
  const issues = [];
  if (!item) return [`missing lifecycle scenario: ${scenario}`];
  if (item.error) return [`${item.name}: invalid JSON`];
  const x = item.data;
  if (x.schemaVersion !== 1) issues.push(`${scenario}: schemaVersion must be 1`);
  if (!validTime(x.observedAtUtc)) issues.push(`${scenario}: observedAtUtc is invalid`);
  if (x.preInstallEvidenceSha256 !== preHash) issues.push(`${scenario}: pre-install evidence hash mismatch`);
  if (x.voxveilCommit !== pre.voxveilCommit) issues.push(`${scenario}: commit mismatch`);
  if (x.releaseChannel !== pre.releaseChannel) issues.push(`${scenario}: release-channel mismatch`);
  if (x.architecture !== pre.architecture) issues.push(`${scenario}: architecture mismatch`);
  if (x.scenario !== scenario) issues.push(`${scenario}: scenario identity mismatch`);
  if (x.result !== 'pass') issues.push(`${scenario}: latest result is ${x.result ?? 'missing'}, not pass`);
  if (typeof x.method !== 'string' || !x.method.trim()) issues.push(`${scenario}: method is missing`);
  if (typeof x.notes !== 'string' || !x.notes.trim()) issues.push(`${scenario}: notes are missing`);
  if (x?.machine?.windowsBuild !== pre?.machine?.windowsBuild ||
      x?.machine?.windowsVersion !== pre?.machine?.windowsVersion ||
      x?.machine?.nativeArchitecture !== pre?.machine?.nativeArchitecture) {
    issues.push(`${scenario}: machine build/version/architecture differs from pre-install evidence`);
  }
  if (x?.machine?.secureBootEnabled !== true) issues.push(`${scenario}: Secure Boot was not recorded enabled`);
  if (x?.machine?.testSigningEnabled !== false) issues.push(`${scenario}: TESTSIGNING was not recorded off`);
  if (typeof x?.machine?.bootMarker !== 'string' || !x.machine.bootMarker) issues.push(`${scenario}: boot marker is missing`);
  for (const field of ['infSha256', 'catalogSha256', 'driverSha256', 'submissionManifestSha256']) {
    if (x?.package?.[field] !== pre?.package?.[field]) issues.push(`${scenario}: package.${field} mismatch`);
  }
  if (!Array.isArray(x.evidenceFiles)) issues.push(`${scenario}: evidenceFiles must be an array`);
  else {
    for (const file of x.evidenceFiles) {
      if (typeof file.fileName !== 'string' || !file.fileName ||
          !validHash(file.sha256) ||
          !Number.isInteger(file.bytes) || file.bytes < 0) {
        issues.push(`${scenario}: evidenceFiles contains an invalid descriptor`);
        break;
      }
    }
  }
  if (scenario === 'reboot-resume') {
    if (!validHash(x.priorLifecycleEvidenceSha256)) issues.push('reboot-resume: prior lifecycle evidence hash is invalid');
    if (typeof x.priorScenario !== 'string' || !x.priorScenario) issues.push('reboot-resume: prior scenario is missing');
    if (typeof x.priorBootMarker !== 'string' || !x.priorBootMarker) issues.push('reboot-resume: prior boot marker is missing');
    if (x.priorBootMarker === x?.machine?.bootMarker) issues.push('reboot-resume: boot marker did not change');
  }
  return issues;
}

function validateQualification(item, pre, preHash) {
  const issues = [];
  if (!item) return ['missing Retail qualification evidence'];
  if (item.error) return [`${item.name}: invalid JSON`];
  const x = item.data;
  if (x.schemaVersion !== 1) issues.push('qualification: schemaVersion must be 1');
  if (!validTime(x.observedAtUtc)) issues.push('qualification: observedAtUtc is invalid');
  if (x.preInstallEvidenceSha256 !== preHash) issues.push('qualification: pre-install evidence hash mismatch');
  if (x.voxveilCommit !== pre.voxveilCommit) issues.push('qualification: commit mismatch');
  if (x.releaseChannel !== 'retail') issues.push('qualification: releaseChannel must be retail');
  if (x.architecture !== pre.architecture) issues.push('qualification: architecture mismatch');
  if (x.qualificationType !== pre?.package?.signingPath) issues.push('qualification: type does not match pre-install signingPath');
  if (x.result !== 'pass') issues.push(`qualification: latest result is ${x.result ?? 'missing'}, not pass`);
  if (typeof x.method !== 'string' || !x.method.trim()) issues.push('qualification: method is missing');
  if (typeof x.notes !== 'string' || !x.notes.trim()) issues.push('qualification: notes are missing');
  if (!Array.isArray(x.evidenceFiles) || x.evidenceFiles.length === 0) issues.push('qualification: at least one evidence artifact is required');
  else {
    for (const file of x.evidenceFiles) {
      if (typeof file.fileName !== 'string' || !file.fileName ||
          !validHash(file.sha256) ||
          !Number.isInteger(file.bytes) || file.bytes <= 0) {
        issues.push('qualification: evidenceFiles contains an invalid descriptor');
        break;
      }
    }
  }
  for (const field of ['infSha256', 'catalogSha256', 'driverSha256', 'submissionManifestSha256']) {
    if (x?.package?.[field] !== pre?.package?.[field]) issues.push(`qualification: package.${field} mismatch`);
  }
  return issues;
}

export function auditWindowsDriverEvidence(args) {
  const measurements = resolve(args.workspace, 'measurements');
  const preItems = jsonFiles(measurements, 'signed-driver-preinstall-');
  const lifecycleItems = jsonFiles(measurements, 'signed-driver-lifecycle-');
  const qualificationItems = jsonFiles(measurements, 'signed-driver-qualification-');

  const matchingPre = preItems.filter((item) =>
    !item.error &&
    item.data?.voxveilCommit === args.commit &&
    item.data?.architecture === args.architecture &&
    item.data?.releaseChannel === args.releaseChannel
  );

  if (matchingPre.length === 0) {
    return {
      status: 'incomplete',
      commit: args.commit,
      architecture: args.architecture,
      releaseChannel: args.releaseChannel,
      issues: ['no matching signed-driver pre-install evidence'],
    };
  }

  const candidates = matchingPre
    .sort((a, b) => Date.parse(b.data.generatedAtUtc ?? 0) - Date.parse(a.data.generatedAtUtc ?? 0))
    .map((preItem) => {
      const pre = preItem.data;
      const preIssues = validatePreInstall(preItem, args);
      const preHash = sha256(preItem.path);
      const boundLifecycle = lifecycleItems.filter((item) => !item.error && item.data?.preInstallEvidenceSha256 === preHash);
      const scenarioRecords = {};
      const issues = [...preIssues];

      for (const scenario of REQUIRED_SCENARIOS) {
        const latest = latestByTime(boundLifecycle.filter((item) => item.data?.scenario === scenario));
        scenarioRecords[scenario] = latest?.name ?? null;
        issues.push(...validateLifecycle(latest, pre, preHash, scenario));
      }

      let qualification = null;
      if (args.releaseChannel === 'retail') {
        const latest = latestByTime(
          qualificationItems.filter((item) => !item.error && item.data?.preInstallEvidenceSha256 === preHash)
        );
        qualification = latest?.name ?? null;
        issues.push(...validateQualification(latest, pre, preHash));
      }

      return {
        status: issues.length === 0 ? 'complete' : 'incomplete',
        preInstallEvidence: preItem.name,
        preInstallEvidenceSha256: preHash,
        scenarioRecords,
        qualification,
        issues,
      };
    });

  const complete = candidates.find((candidate) => candidate.status === 'complete');
  const chosen = complete ?? candidates[0];
  return {
    status: chosen.status,
    commit: args.commit,
    architecture: args.architecture,
    releaseChannel: args.releaseChannel,
    ...chosen,
  };
}

if (import.meta.url === `file://${process.argv[1].replaceAll('\\', '/')}` ||
    process.argv[1]?.endsWith('audit-windows-driver-release-evidence.mjs')) {
  try {
    const args = parseArgs(process.argv.slice(2));
    const result = auditWindowsDriverEvidence(args);
    if (args.json) {
      process.stdout.write(JSON.stringify(result, null, 2) + '\n');
    } else {
      process.stdout.write(`Windows driver release evidence: ${result.status}\n`);
      if (result.preInstallEvidence) process.stdout.write(`Pre-install: ${result.preInstallEvidence}\n`);
      for (const issue of result.issues ?? []) process.stdout.write(`- ${issue}\n`);
    }
    process.exitCode = result.status === 'complete' ? 0 : 1;
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 2;
  }
}
