#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';

const RATES = [44100, 48000];
const PROFILES = ['music-preservation', 'balanced'];
const HASH_RE = /^[a-f0-9]{64}$/i;

export const manualCoverageChecks = [
  'male and female lead-vocal coverage',
  'sparse and dense accompaniment coverage',
  'strong centered low-frequency/instrument coverage',
  'wide stereo ambience/reverb coverage',
  'mono or near-mono coverage',
  'harmony/double-tracked vocal coverage where licensing permits',
];

async function readJson(file) {
  return JSON.parse(await readFile(file, 'utf8'));
}

async function fileSha256(file) {
  return createHash('sha256').update(await readFile(file)).digest('hex');
}

async function listJson(dir) {
  try {
    return (await readdir(dir, { withFileTypes: true }))
      .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
      .map((entry) => path.join(dir, entry.name))
      .sort();
  } catch (error) {
    if (error?.code === 'ENOENT') return [];
    throw error;
  }
}

function hashOk(value) {
  return typeof value === 'string' && HASH_RE.test(value);
}

function nonEmpty(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

function renderEvidenceIssues(manifest, evidence, label) {
  const issues = [];
  const rate = manifest.targetSampleRate;
  if (evidence.fixtureId !== manifest.fixtureId) issues.push(`${label}: fixtureId does not match manifest`);
  if (evidence.tier !== manifest.tier) issues.push(`${label}: tier does not match manifest`);
  if (evidence.sampleRate !== rate) issues.push(`${label}: sampleRate does not match manifest`);
  if (evidence.vocal !== 0) issues.push(`${label}: acceptance render must use Vocal = 0`);
  if (evidence.objectiveStatus !== 'rendered') issues.push(`${label}: objectiveStatus is not rendered`);
  if (evidence.frameCountParity !== true) issues.push(`${label}: frameCountParity is not true`);
  if (evidence?.source?.nativeSampleRate !== rate) issues.push(`${label}: source nativeSampleRate does not match target rate`);
  if (!hashOk(evidence?.source?.sha256)) issues.push(`${label}: source SHA-256 is missing/invalid`);
  if (!hashOk(evidence?.input?.sha256)) issues.push(`${label}: input SHA-256 is missing/invalid`);
  if (!hashOk(evidence?.renders?.musicPreservation?.sha256)) issues.push(`${label}: Music preservation SHA-256 is missing/invalid`);
  if (!hashOk(evidence?.renders?.balanced?.sha256)) issues.push(`${label}: Balanced SHA-256 is missing/invalid`);
  if (evidence?.renders?.strong && !hashOk(evidence.renders.strong.sha256)) {
    issues.push(`${label}: Strong SHA-256 is invalid`);
  }
  if (evidence?.subjectiveReview?.status !== 'complete') issues.push(`${label}: subjective review is incomplete`);
  if (evidence?.subjectiveReview?.decision !== 'accepted') issues.push(`${label}: subjective review is not accepted`);
  if (!nonEmpty(evidence?.subjectiveReview?.musicPreservation)) issues.push(`${label}: Music preservation listening notes are missing`);
  if (!nonEmpty(evidence?.subjectiveReview?.balanced)) issues.push(`${label}: Balanced listening notes are missing`);
  if (!nonEmpty(evidence?.subjectiveReview?.reviewMethod)) issues.push(`${label}: listening review method is missing`);
  return issues;
}

function manifestIssues(manifest, fileName) {
  const issues = [];
  const label = `manifest ${fileName}`;
  if (!nonEmpty(manifest.fixtureId)) issues.push(`${label}: fixtureId is missing`);
  if (!['controlled', 'natural-mix'].includes(manifest.tier)) issues.push(`${label}: tier is invalid`);
  if (!RATES.includes(manifest.targetSampleRate)) issues.push(`${label}: targetSampleRate is invalid`);
  if (!hashOk(manifest.fixtureSha256)) issues.push(`${label}: fixture SHA-256 is missing/invalid`);
  if (!Array.isArray(manifest.sources) || manifest.sources.length === 0) {
    issues.push(`${label}: sources are missing`);
  } else {
    for (const source of manifest.sources) {
      if (!nonEmpty(source.role) || !nonEmpty(source.sourceRecord) || !nonEmpty(source.sourceFile) ||
          !nonEmpty(source.license) || !/^\d{4}-\d{2}-\d{2}$/.test(source.licenseCheckedOn ?? '') ||
          !hashOk(source.sourceSha256)) {
        issues.push(`${label}: source provenance is incomplete for role ${source.role ?? '<unknown>'}`);
      }
    }
  }

  if (manifest.tier === 'controlled') {
    if (!manifest.mixRecipe || manifest.mixRecipe.normalization !== 'none') {
      issues.push(`${label}: controlled fixture must have a non-normalizing mix recipe`);
    }
    const vocal = manifest.sources?.find((source) => source.role === 'vocal');
    const accompaniment = manifest.sources?.find((source) => source.role === 'accompaniment');
    if (vocal?.dataset !== 'VocalSet') issues.push(`${label}: controlled vocal source is not VocalSet`);
    if (accompaniment?.dataset !== 'URMP') issues.push(`${label}: controlled accompaniment source is not URMP`);
    for (const role of ['vocal', 'accompaniment']) {
      const reference = manifest.referenceFiles?.[role];
      if (!reference || !nonEmpty(reference.rawFile) || !hashOk(reference.sha256) ||
          !Number.isInteger(reference.bytes) || reference.bytes < 8) {
        issues.push(`${label}: controlled ${role} aligned reference is missing/invalid`);
      }
    }
  } else if (manifest.mixRecipe !== null) {
    issues.push(`${label}: natural-mix fixture must have mixRecipe = null`);
  }

  return issues;
}

function runtimeRequirementKey(rate, profile, workload) {
  return `${rate}/${profile}/${workload}`;
}

function machineRouteKey(evidence) {
  const machine = evidence.machine ?? {};
  const route = evidence.route ?? {};
  return JSON.stringify([
    machine.windowsBuild ?? '',
    machine.osArchitecture ?? '',
    machine.logicalProcessorCount ?? '',
    ...(Array.isArray(machine.cpuModels) ? machine.cpuModels : []),
    route.sourceEndpoint ?? '',
    route.physicalOutput ?? '',
  ]);
}

function runtimeRecordValid(evidence, workload) {
  const route = evidence.route ?? {};
  const cpu = evidence.cpu ?? {};
  if (!nonEmpty(route.sourceEndpoint) || !nonEmpty(route.physicalOutput)) return false;
  if (route.vocal !== 0 || !RATES.includes(route.sampleRate) || !PROFILES.includes(route.profile)) return false;
  if (cpu.workloadState !== workload || cpu.status !== 'sampled') return false;
  if (typeof cpu.normalizedPercent !== 'number' || !Number.isFinite(cpu.normalizedPercent) || cpu.normalizedPercent < 0) return false;

  if (workload === 'processing') {
    const manual = evidence.manualMeasurements ?? {};
    return manual.status === 'complete' &&
      Number.isInteger(manual.dropoutCount) && manual.dropoutCount >= 0 &&
      typeof manual.endToEndLatencyMs === 'number' && Number.isFinite(manual.endToEndLatencyMs) && manual.endToEndLatencyMs >= 0 &&
      nonEmpty(manual.latencyMethod);
  }

  return true;
}

export async function auditWorkspace(workspaceRoot) {
  const workspace = path.resolve(workspaceRoot);
  const manifestsDir = path.join(workspace, 'manifests');
  const measurementsDir = path.join(workspace, 'measurements');
  const issues = [];
  const warnings = [];

  const manifestFiles = await listJson(manifestsDir);
  const manifests = [];
  for (const file of manifestFiles) {
    try {
      const manifest = await readJson(file);
      issues.push(...manifestIssues(manifest, path.basename(file)));
      manifests.push({ file, manifest });
    } catch (error) {
      issues.push(`manifest ${path.basename(file)}: invalid JSON (${error.message})`);
    }
  }

  const controlledAcceptedByRate = new Map(RATES.map((rate) => [rate, 0]));
  const acceptedFixtures = new Map();
  let naturalAccepted = 0;

  for (const { manifest } of manifests) {
    if (!nonEmpty(manifest.fixtureId) || !RATES.includes(manifest.targetSampleRate) ||
        !['controlled', 'natural-mix'].includes(manifest.tier)) {
      continue;
    }

    const evidenceFile = path.join(
      measurementsDir,
      `${manifest.fixtureId}-${manifest.targetSampleRate}-render-evidence.json`,
    );

    let evidence;
    try {
      evidence = await readJson(evidenceFile);
    } catch (error) {
      issues.push(`${manifest.fixtureId}: render evidence missing/invalid (${error.code === 'ENOENT' ? 'not found' : error.message})`);
      continue;
    }

    const evidenceIssues = renderEvidenceIssues(manifest, evidence, manifest.fixtureId);
    issues.push(...evidenceIssues);
    if (evidenceIssues.length !== 0) continue;

    if (manifest.tier === 'controlled') {
      const metricsFile = path.join(measurementsDir, `${manifest.fixtureId}-${manifest.targetSampleRate}-controlled-metrics.json`);
      let metrics;
      try {
        metrics = await readJson(metricsFile);
      } catch (error) {
        issues.push(`${manifest.fixtureId}: controlled quantitative metrics missing/invalid (${error.code === 'ENOENT' ? 'not found' : error.message})`);
        continue;
      }
      const metricIssues = [];
      if (metrics.fixtureId !== manifest.fixtureId || metrics.tier !== 'controlled' ||
          metrics.sampleRate !== manifest.targetSampleRate || metrics.vocal !== 0) {
        metricIssues.push(`${manifest.fixtureId}: controlled metrics identity does not match manifest`);
      }
      for (const [profileName, renderKey] of [['Music preservation', 'musicPreservation'], ['Balanced', 'balanced']]) {
        const profile = metrics.renders?.[renderKey];
        for (const key of ['vocalAttenuationDb', 'accompanimentGainChangeDb', 'accompanimentErrorRelativeDb', 'unexplainedResidualRms']) {
          if (typeof profile?.metrics?.[key] !== 'number' || !Number.isFinite(profile.metrics[key])) {
            metricIssues.push(`${manifest.fixtureId}: ${profileName} controlled metric ${key} is missing/non-finite`);
          }
        }
      }
      issues.push(...metricIssues);
      if (metricIssues.length !== 0) continue;
      controlledAcceptedByRate.set(
        manifest.targetSampleRate,
        (controlledAcceptedByRate.get(manifest.targetSampleRate) ?? 0) + 1,
      );
      acceptedFixtures.set(manifest.fixtureId, {
        manifestFile: path.join(manifestsDir, `${manifest.fixtureId}.json`),
        renderEvidenceFile: evidenceFile,
      });
    } else {
      if (manifest.status === 'candidate') {
        issues.push(`${manifest.fixtureId}: candidate natural mix cannot count toward acceptance`);
      } else {
        naturalAccepted += 1;
        acceptedFixtures.set(manifest.fixtureId, {
          manifestFile: path.join(manifestsDir, `${manifest.fixtureId}.json`),
          renderEvidenceFile: evidenceFile,
        });
      }
    }
  }

  for (const rate of RATES) {
    const count = controlledAcceptedByRate.get(rate) ?? 0;
    if (count < 2) {
      issues.push(`controlled matrix: need at least 2 accepted ${rate} Hz fixtures; found ${count}`);
    }
  }
  if (naturalAccepted < 1) {
    issues.push('natural-mix matrix: need at least 1 accepted non-candidate natural mix; found 0');
  }

  const coverageReviewFiles = (await listJson(measurementsDir))
    .filter((file) => path.basename(file).startsWith('classic-dsp-coverage-review-'));
  let semanticCoverageComplete = false;
  let coverageReviewFile = null;
  if (coverageReviewFiles.length === 0) {
    issues.push('semantic coverage: explicit human coverage review is missing');
  } else {
    const reviews = [];
    for (const file of coverageReviewFiles) {
      try {
        reviews.push({ file, review: await readJson(file) });
      } catch (error) {
        issues.push(`semantic coverage ${path.basename(file)}: invalid JSON (${error.message})`);
      }
    }
    reviews.sort((a, b) => Date.parse(b.review.reviewedAtUtc ?? 0) - Date.parse(a.review.reviewedAtUtc ?? 0));
    const latest = reviews[0];
    if (latest) {
      coverageReviewFile = path.basename(latest.file);
      const reviewIssues = [];
      if (latest.review.schemaVersion !== 1) reviewIssues.push('schemaVersion must be 1');
      if (!nonEmpty(latest.review.reviewMethod)) reviewIssues.push('reviewMethod is missing');
      if (!nonEmpty(latest.review.notes)) reviewIssues.push('notes are missing');
      if (!Number.isFinite(Date.parse(latest.review.reviewedAtUtc ?? ''))) reviewIssues.push('reviewedAtUtc is invalid');

      const requiredCategories = [
        'maleLeadVocal',
        'femaleLeadVocal',
        'sparseAccompaniment',
        'denseAccompaniment',
        'centeredLowFrequencyOrInstrument',
        'wideStereoAmbience',
        'monoNearMono',
      ];
      for (const category of requiredCategories) {
        const entry = latest.review.categories?.[category];
        if (entry?.status !== 'covered' || !nonEmpty(entry.fixtureId)) {
          reviewIssues.push(`${category}: accepted fixture mapping is missing`);
          continue;
        }
        const accepted = acceptedFixtures.get(entry.fixtureId);
        if (!accepted) {
          reviewIssues.push(`${category}: fixture ${entry.fixtureId} is not in the accepted evidence set`);
          continue;
        }
        const manifestHash = await fileSha256(accepted.manifestFile);
        const renderHash = await fileSha256(accepted.renderEvidenceFile);
        if (entry.manifestSha256 !== manifestHash || entry.renderEvidenceSha256 !== renderHash) {
          reviewIssues.push(`${category}: fixture evidence hashes no longer match the review`);
        }
      }

      const harmony = latest.review.categories?.harmonyDoubleTracked;
      if (harmony?.status === 'covered') {
        if (!nonEmpty(harmony.fixtureId)) {
          reviewIssues.push('harmonyDoubleTracked: fixture mapping is missing');
        } else {
          const accepted = acceptedFixtures.get(harmony.fixtureId);
          if (!accepted) {
            reviewIssues.push(`harmonyDoubleTracked: fixture ${harmony.fixtureId} is not in the accepted evidence set`);
          } else {
            const manifestHash = await fileSha256(accepted.manifestFile);
            const renderHash = await fileSha256(accepted.renderEvidenceFile);
            if (harmony.manifestSha256 !== manifestHash || harmony.renderEvidenceSha256 !== renderHash) {
              reviewIssues.push('harmonyDoubleTracked: fixture evidence hashes no longer match the review');
            }
          }
        }
      } else if (harmony?.status === 'not-applicable') {
        if (!nonEmpty(harmony.reason)) reviewIssues.push('harmonyDoubleTracked: not-applicable reason is missing');
      } else {
        reviewIssues.push('harmonyDoubleTracked: coverage or licensing-based not-applicable decision is missing');
      }

      issues.push(...reviewIssues.map((issue) => `semantic coverage: ${issue}`));
      semanticCoverageComplete = reviewIssues.length === 0;
    }
  }

  const measurementFiles = await listJson(measurementsDir);
  const runtimeRecords = [];
  for (const file of measurementFiles) {
    if (!path.basename(file).startsWith('windows-runtime-')) continue;
    try {
      const evidence = await readJson(file);
      if (evidence?.route && evidence?.machine && evidence?.cpu) {
        runtimeRecords.push({ file, evidence });
      }
    } catch (error) {
      issues.push(`runtime evidence ${path.basename(file)}: invalid JSON (${error.message})`);
    }
  }

  const groups = new Map();
  for (const record of runtimeRecords) {
    const key = machineRouteKey(record.evidence);
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(record);
  }

  const requiredRuntimeKeys = [];
  for (const rate of RATES) {
    for (const profile of PROFILES) {
      requiredRuntimeKeys.push(runtimeRequirementKey(rate, profile, 'idle'));
      requiredRuntimeKeys.push(runtimeRequirementKey(rate, profile, 'processing'));
    }
  }

  let runtimeMatrixComplete = false;
  let bestMissing = requiredRuntimeKeys;
  for (const records of groups.values()) {
    const satisfied = new Set();
    for (const { evidence } of records) {
      const rate = evidence?.route?.sampleRate;
      const profile = evidence?.route?.profile;
      const workload = evidence?.cpu?.workloadState;
      if (RATES.includes(rate) && PROFILES.includes(profile) && ['idle', 'processing'].includes(workload) &&
          runtimeRecordValid(evidence, workload)) {
        satisfied.add(runtimeRequirementKey(rate, profile, workload));
      }
    }
    const missing = requiredRuntimeKeys.filter((key) => !satisfied.has(key));
    if (missing.length < bestMissing.length) bestMissing = missing;
    if (missing.length === 0) {
      runtimeMatrixComplete = true;
      break;
    }
  }

  if (!runtimeMatrixComplete) {
    issues.push(
      `runtime matrix: no single machine/route group has complete idle+processing evidence for both profiles at 44.1/48 kHz; missing in best group: ${bestMissing.join(', ')}`,
    );
  }

  if (manifestFiles.length === 0) warnings.push('no fixture manifests were found');
  if (runtimeRecords.length === 0) warnings.push('no Windows runtime evidence files were found');

  return {
    workspace,
    structuralComplete: issues.length === 0,
    summary: {
      controlledAccepted44100: controlledAcceptedByRate.get(44100) ?? 0,
      controlledAccepted48000: controlledAcceptedByRate.get(48000) ?? 0,
      naturalMixAccepted: naturalAccepted,
      runtimeMatrixComplete,
      semanticCoverageComplete,
      coverageReviewFile,
    },
    issues,
    warnings,
    manualCoverageChecks,
  };
}

function parseArgs(argv) {
  let workspace = '.local-evaluation/classic-dsp';
  let json = false;
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--workspace') {
      workspace = argv[index + 1];
      index += 1;
    } else if (arg === '--json') {
      json = true;
    } else if (arg === '--help' || arg === '-h') {
      return { help: true, workspace, json };
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return { workspace, json, help: false };
}

function printHuman(report) {
  console.log(report.structuralComplete ? 'Structural acceptance evidence: complete' : 'Structural acceptance evidence: incomplete');
  console.log(`Controlled accepted: 44.1 kHz=${report.summary.controlledAccepted44100}, 48 kHz=${report.summary.controlledAccepted48000}`);
  console.log(`Natural mixes accepted: ${report.summary.naturalMixAccepted}`);
  console.log(`Runtime matrix complete: ${report.summary.runtimeMatrixComplete}`);
  console.log(`Semantic coverage review complete: ${report.summary.semanticCoverageComplete}`);
  for (const issue of report.issues) console.log(`ERROR: ${issue}`);
  for (const warning of report.warnings) console.log(`WARN: ${warning}`);
  if (!report.summary.semanticCoverageComplete) {
    console.log('Semantic coverage categories requiring explicit human review:');
    for (const item of report.manualCoverageChecks) console.log(`- ${item}`);
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log('Usage: node scripts/evaluation/audit-classic-dsp-evidence.mjs [--workspace <path>] [--json]');
    return;
  }
  const report = await auditWorkspace(args.workspace);
  if (args.json) console.log(JSON.stringify(report, null, 2));
  else printHuman(report);
  process.exitCode = report.structuralComplete ? 0 : 1;
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : '';
if (invokedPath === import.meta.url) {
  await main();
}
