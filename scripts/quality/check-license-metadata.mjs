import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { approvalErrors } from '../lib/dependency-policy.mjs';

const DEPENDENCY_FIELDS = ['dependencies', 'devDependencies', 'optionalDependencies'];
const WINDOWS_DRIVER_SAMPLES_REVISION = '67d81f217bc01edf7a4320e4911c11065635acfa';
const WINDOWS_DRIVER_SAMPLES_SYSVAD_TREE = '6fa502f5bfb3de1395a6c9ffe71e322fd9e28926';
const WINDOWS_DRIVER_SAMPLES_ROOT = 'third_party/microsoft/windows-driver-samples';

async function readJson(file) {
  return JSON.parse(await readFile(file, 'utf8'));
}

function dependencyEntries(manifest) {
  return DEPENDENCY_FIELDS.flatMap((field) => Object.entries(manifest[field] ?? {}));
}

function isExactVersion(version) {
  return /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version);
}

async function expandWorkspace(root, pattern) {
  const segments = pattern.split('/').filter(Boolean);
  if (segments.some((segment) => segment.includes('**'))) {
    throw new Error(`unsupported recursive workspace pattern: ${pattern}`);
  }

  let paths = [''];
  for (const segment of segments) {
    if (segment === '*') {
      const expanded = [];
      for (const current of paths) {
        const dir = path.join(root, current);
        const entries = await readdir(dir, { withFileTypes: true });
        for (const entry of entries) {
          if (entry.isDirectory()) expanded.push(path.join(current, entry.name));
        }
      }
      paths = expanded;
    } else if (segment.includes('*')) {
      throw new Error(`unsupported workspace pattern segment: ${segment}`);
    } else {
      paths = paths.map((current) => path.join(current, segment));
    }
  }
  return paths.map((workspace) => path.join(workspace, 'package.json'));
}

async function manifestsFor(root) {
  const rootManifest = await readJson(path.join(root, 'package.json'));
  const manifests = [{ file: 'package.json', value: rootManifest }];
  for (const workspace of rootManifest.workspaces ?? []) {
    for (const file of await expandWorkspace(root, workspace)) {
      manifests.push({ file, value: await readJson(path.join(root, file)) });
    }
  }
  return manifests;
}

export async function auditDependencyMetadata(root) {
  const allowlistFile = path.join(root, 'docs/specs/security/dependency-allowlist.json');
  const allowlist = (await readJson(allowlistFile)).packages ?? {};
  const manifests = await manifestsFor(root);
  const errors = [];
  const seen = new Map();

  for (const { file, value } of manifests) {
    for (const [name, version] of dependencyEntries(value)) {
      if (!isExactVersion(version)) {
        errors.push(`${file}: ${name} must use an exact version, got ${version}`);
      }
      const previous = seen.get(name);
      if (previous && previous !== version) {
        errors.push(`${name} uses conflicting versions ${previous} and ${version}`);
      }
      seen.set(name, version);
      const approved = allowlist[name];
      if (!approved) {
        errors.push(`${file}: ${name} is not allowlisted`);
        continue;
      }
      if (approved.version !== version) {
        errors.push(`${file}: ${name} uses ${version}; allowlist approves ${approved.version}`);
      }
      errors.push(...approvalErrors(name, approved));
    }
  }

  for (const name of Object.keys(allowlist)) {
    if (!seen.has(name)) errors.push(`${name} is allowlisted but not declared by a workspace package`);
  }
  return errors;
}

async function readRequiredText(root, relativePath, errors) {
  try {
    return await readFile(path.join(root, relativePath), 'utf8');
  } catch {
    errors.push(`${relativePath}: required third-party metadata file is missing`);
    return null;
  }
}

export async function auditThirdPartyMetadata(root) {
  const errors = [];
  const revisionPath = `${WINDOWS_DRIVER_SAMPLES_ROOT}/SOURCE_REVISION`;
  const treePath = `${WINDOWS_DRIVER_SAMPLES_ROOT}/SYSVAD_TREE_SHA`;
  const readmePath = `${WINDOWS_DRIVER_SAMPLES_ROOT}/README.voxveil.md`;
  const licensePath = `${WINDOWS_DRIVER_SAMPLES_ROOT}/LICENSE.txt`;

  const revision = await readRequiredText(root, revisionPath, errors);
  const sysvadTree = await readRequiredText(root, treePath, errors);
  const readme = await readRequiredText(root, readmePath, errors);
  const license = await readRequiredText(root, licensePath, errors);

  if (revision !== null && revision.trim() !== WINDOWS_DRIVER_SAMPLES_REVISION) {
    errors.push(
      `${WINDOWS_DRIVER_SAMPLES_ROOT}: revision must remain pinned to ${WINDOWS_DRIVER_SAMPLES_REVISION}`,
    );
  }
  if (sysvadTree !== null && sysvadTree.trim() !== WINDOWS_DRIVER_SAMPLES_SYSVAD_TREE) {
    errors.push(
      `${WINDOWS_DRIVER_SAMPLES_ROOT}/audio/sysvad tree must remain pinned to ${WINDOWS_DRIVER_SAMPLES_SYSVAD_TREE}`,
    );
  }
  if (readme !== null) {
    if (!/microsoft\/Windows-driver-samples/i.test(readme)) {
      errors.push(`${readmePath}: upstream microsoft/Windows-driver-samples attribution is missing`);
    }
    if (!/audio\/sysvad/i.test(readme)) {
      errors.push(`${readmePath}: imported audio/sysvad scope is missing`);
    }
    if (!readme.includes(WINDOWS_DRIVER_SAMPLES_SYSVAD_TREE)) {
      errors.push(`${readmePath}: pinned audio/sysvad tree identity is missing`);
    }
  }
  if (license !== null && !license.startsWith('The Microsoft Public License (MS-PL)')) {
    errors.push(`${licensePath}: expected the Microsoft Public License (MS-PL)`);
  }

  return errors;
}

async function main() {
  const root = path.resolve(process.argv[2] ?? '.');
  const errors = [
    ...(await auditDependencyMetadata(root)),
    ...(await auditThirdPartyMetadata(root)),
  ];
  if (errors.length) {
    console.error(errors.map((error) => `FAIL ${error}`).join('\n'));
    process.exitCode = 1;
    return;
  }
  console.log('Dependency and third-party metadata gates passed.');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await main();
}
