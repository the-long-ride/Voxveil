#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';
import { auditReleaseReadiness } from './audit-release-readiness.mjs';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function stableValue(value) {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value).sort().map((key) => [key, stableValue(value[key])]),
    );
  }
  return value;
}

function stableJson(value) {
  return JSON.stringify(stableValue(value));
}

async function inventoryJsonFiles(root, domain) {
  const absolute = path.resolve(root);
  const files = [];

  async function walk(relative = '') {
    const directory = path.join(absolute, relative);
    let entries;
    try {
      entries = await readdir(directory, { withFileTypes: true });
    } catch (error) {
      if (error?.code === 'ENOENT') return;
      throw error;
    }

    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      const child = relative ? path.join(relative, entry.name) : entry.name;
      if (entry.isDirectory()) {
        await walk(child);
      } else if (entry.isFile() && entry.name.endsWith('.json')) {
        const bytes = await readFile(path.join(absolute, child));
        files.push({
          domain,
          relativePath: child.replaceAll('\\', '/'),
          sha256: sha256(bytes),
          bytes: bytes.length,
        });
      }
    }
  }

  await walk();
  return files;
}

async function criticalPackageFiles(packageRoot) {
  const root = path.resolve(packageRoot);
  const files = [];
  for (const relativePath of ['release-manifest.json', 'SHA256SUMS.txt']) {
    const bytes = await readFile(path.join(root, relativePath));
    files.push({
      relativePath,
      sha256: sha256(bytes),
      bytes: bytes.length,
    });
  }
  return files;
}

export async function buildReleaseEvidenceManifest(args, dependencies = {}) {
  const releaseAudit = dependencies.releaseAudit ?? auditReleaseReadiness;
  const now = dependencies.now ?? (() => new Date());
  const report = await releaseAudit(args);

  if (report?.status !== 'complete') {
    const detail = (report?.issues ?? []).join('; ');
    throw new Error(`Refusing to export release evidence because the unified audit is incomplete${detail ? `: ${detail}` : ''}.`);
  }

  const evidenceFiles = [
    ...await inventoryJsonFiles(args.classicWorkspace, 'classic-dsp'),
    ...await inventoryJsonFiles(args.driverWorkspace, 'windows-driver'),
    ...await inventoryJsonFiles(args.apoWorkspace, 'windows-apo'),
  ].sort((a, b) =>
    a.domain.localeCompare(b.domain) || a.relativePath.localeCompare(b.relativePath)
  );

  const packageFiles = await criticalPackageFiles(args.packageRoot);

  const core = {
    schemaVersion: 1,
    voxveilCommit: args.commit,
    architecture: args.architecture,
    releaseChannel: args.releaseChannel,
    package: {
      signedApoPresent: report.windowsPackage?.signedApoPresent === true,
      signedVirtualDriverPresent: report.windowsPackage?.signedVirtualDriverPresent === true,
      filesChecked: report.windowsPackage?.filesChecked ?? null,
      criticalFiles: packageFiles,
    },
    auditStatus: {
      windowsPackage: report.windowsPackage?.status ?? null,
      windowsApo: report.windowsApo?.status ?? null,
      classicDsp: report.classicDsp?.structuralComplete === true ? 'complete' : 'incomplete',
      windowsDriver: report.windowsDriver?.status ?? null,
    },
    evidenceFiles,
    selectedEvidence: {
      classicDspCoverageReview: report.classicDsp?.summary?.coverageReviewFile
        ? path.basename(report.classicDsp.summary.coverageReviewFile)
        : null,
      windowsDriverPreInstall: report.windowsDriver?.preInstallEvidence ?? null,
      windowsDriverLifecycle: report.windowsDriver?.scenarioRecords ?? {},
      windowsDriverQualification: report.windowsDriver?.qualification ?? null,
      windowsApoScenarios: report.windowsApo?.scenarioRecords ?? {},
      windowsApoQualification: report.windowsApo?.qualification ?? null,
    },
    statement: 'This manifest inventories the exact local JSON evidence set and critical final-package metadata accepted by the unified release audit. External artifacts remain represented by the file-name/size/SHA-256 descriptors stored in their evidence records.',
  };

  return {
    ...core,
    evidenceSetSha256: sha256(Buffer.from(stableJson(core), 'utf8')),
    createdAtUtc: now().toISOString(),
  };
}

function parseArgs(argv) {
  const args = {
    commit: null,
    architecture: 'x64',
    releaseChannel: 'pilot',
    classicWorkspace: '.local-evaluation/classic-dsp',
    driverWorkspace: '.local-evaluation/windows-driver',
    apoWorkspace: '.local-evaluation/windows-apo',
    packageRoot: null,
    output: null,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    const value = argv[index + 1];
    if (!value) throw new Error(`Missing value for ${token}`);
    if (token === '--commit') args.commit = value.toLowerCase();
    else if (token === '--architecture') args.architecture = value;
    else if (token === '--release-channel') args.releaseChannel = value.toLowerCase();
    else if (token === '--classic-workspace') args.classicWorkspace = value;
    else if (token === '--driver-workspace') args.driverWorkspace = value;
    else if (token === '--apo-workspace') args.apoWorkspace = value;
    else if (token === '--package-root') args.packageRoot = value;
    else if (token === '--output') args.output = value;
    else throw new Error(`Unknown argument: ${token}`);
    index += 1;
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
  if (!args.packageRoot) {
    args.packageRoot = args.architecture === 'ARM64'
      ? 'dist/windows-arm64/Voxveil'
      : 'dist/windows-x64/Voxveil';
  }
  if (!args.output) {
    args.output = path.join(
      'release-metadata',
      `windows-release-evidence-${args.commit}-${args.architecture.toLowerCase()}-${args.releaseChannel}.json`,
    );
  }
  return args;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const manifest = await buildReleaseEvidenceManifest(args);
  const output = path.resolve(args.output);
  await mkdir(path.dirname(output), { recursive: true });
  const bytes = Buffer.from(JSON.stringify(manifest, null, 2) + '\n', 'utf8');
  await writeFile(output, bytes);
  await writeFile(`${output}.sha256`, `${sha256(bytes)}  ${path.basename(output)}\n`, 'ascii');
  process.stdout.write(`Release evidence manifest: ${output}\n`);
  process.stdout.write(`Evidence set SHA-256: ${manifest.evidenceSetSha256}\n`);
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : '';
if (invokedPath === import.meta.url) {
  try {
    await main();
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 2;
  }
}
