import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { approvalErrors } from '../lib/dependency-policy.mjs';

const DEPENDENCY_FIELDS = ['dependencies', 'devDependencies', 'optionalDependencies'];

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

async function main() {
  const root = path.resolve(process.argv[2] ?? '.');
  const errors = await auditDependencyMetadata(root);
  if (errors.length) {
    console.error(errors.map((error) => `FAIL ${error}`).join('\n'));
    process.exitCode = 1;
    return;
  }
  console.log('Dependency metadata gate passed.');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await main();
}
