#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { auditWorkspace as auditClassicDspEvidence } from './audit-classic-dsp-evidence.mjs';
import { auditWindowsDriverEvidence } from './audit-windows-driver-release-evidence.mjs';

function parseArgs(argv) {
  const args = {
    commit: null,
    architecture: 'x64',
    releaseChannel: 'pilot',
    classicWorkspace: '.local-evaluation/classic-dsp',
    driverWorkspace: '.local-evaluation/windows-driver',
    json: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === '--json') {
      args.json = true;
      continue;
    }
    const value = argv[index + 1];
    if (!value) throw new Error(`Missing value for ${token}`);
    if (token === '--commit') args.commit = value.toLowerCase();
    else if (token === '--architecture') args.architecture = value;
    else if (token === '--release-channel') args.releaseChannel = value.toLowerCase();
    else if (token === '--classic-workspace') args.classicWorkspace = value;
    else if (token === '--driver-workspace') args.driverWorkspace = value;
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
  return args;
}

function repositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
}

export function currentCheckoutCommit(repoRoot = repositoryRoot()) {
  return execFileSync('git', ['rev-parse', 'HEAD'], {
    cwd: repoRoot,
    encoding: 'utf8',
    windowsHide: true,
  }).trim().toLowerCase();
}

export async function auditReleaseReadiness(args, dependencies = {}) {
  const checkoutCommit = dependencies.checkoutCommit ?? currentCheckoutCommit();
  const classicAudit = dependencies.classicAudit ?? auditClassicDspEvidence;
  const driverAudit = dependencies.driverAudit ?? auditWindowsDriverEvidence;

  const issues = [];
  if (!/^[a-f0-9]{40}$/.test(checkoutCommit)) {
    issues.push('release checkout: current Git HEAD is invalid');
  } else if (checkoutCommit !== args.commit) {
    issues.push(`release checkout: requested commit ${args.commit} does not match current checkout ${checkoutCommit}`);
  }

  let classicDsp = null;
  let windowsDriver = null;

  if (issues.length === 0) {
    classicDsp = await classicAudit(args.classicWorkspace);
    if (!classicDsp?.structuralComplete) {
      for (const issue of classicDsp?.issues ?? ['Classic DSP evidence is incomplete']) {
        issues.push(`classic-dsp: ${issue}`);
      }
    }

    windowsDriver = driverAudit({
      workspace: args.driverWorkspace,
      commit: args.commit,
      architecture: args.architecture,
      releaseChannel: args.releaseChannel,
    });
    if (windowsDriver?.status !== 'complete') {
      for (const issue of windowsDriver?.issues ?? ['Windows driver evidence is incomplete']) {
        issues.push(`windows-driver: ${issue}`);
      }
    }
  }

  return {
    status: issues.length === 0 ? 'complete' : 'incomplete',
    commit: args.commit,
    checkoutCommit,
    architecture: args.architecture,
    releaseChannel: args.releaseChannel,
    classicDsp,
    windowsDriver,
    issues,
    statement: 'Complete means both repository evidence audits passed for the exact current checkout; it does not replace the underlying human, hardware, licensing, or Microsoft qualification work.',
  };
}

function printHuman(report) {
  console.log(`Release readiness evidence: ${report.status}`);
  console.log(`Requested commit: ${report.commit}`);
  console.log(`Checkout commit: ${report.checkoutCommit}`);
  console.log(`Architecture: ${report.architecture}`);
  console.log(`Release channel: ${report.releaseChannel}`);
  if (report.classicDsp) {
    console.log(`Classic DSP evidence: ${report.classicDsp.structuralComplete ? 'complete' : 'incomplete'}`);
  }
  if (report.windowsDriver) {
    console.log(`Windows driver evidence: ${report.windowsDriver.status}`);
  }
  for (const issue of report.issues) console.log(`ERROR: ${issue}`);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const report = await auditReleaseReadiness(args);
  if (args.json) process.stdout.write(JSON.stringify(report, null, 2) + '\n');
  else printHuman(report);
  process.exitCode = report.status === 'complete' ? 0 : 1;
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
