import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { approvalErrors } from '../lib/dependency-policy.mjs';

const SKIP_DIRS = new Set(['.git', 'target', 'node_modules', 'dist']);
const DEP_SECTION = /(?:^|\.)(?:build-|dev-)?dependencies$/;

async function findManifests(dir, output = []) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    if (entry.isDirectory() && SKIP_DIRS.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) await findManifests(full, output);
    else if (entry.isFile() && entry.name === 'Cargo.toml') output.push(full);
  }
  return output;
}

function parseDependencies(content) {
  let section = '';
  const entries = [];
  for (const raw of content.split(/\r?\n/)) {
    const line = raw.replace(/\s+#.*$/, '').trim();
    const heading = line.match(/^\[([^\]]+)\]$/);
    if (heading) { section = heading[1]; continue; }
    if (!DEP_SECTION.test(section) || !line || line.startsWith('#')) continue;
    const match = line.match(/^([A-Za-z0-9_-]+)\s*=\s*(.+)$/);
    if (match) entries.push({ name: match[1], spec: match[2] });
  }
  return entries;
}

function registryVersion(spec) {
  const simple = spec.match(/^"([^"]+)"$/);
  if (simple) return simple[1];
  const inline = spec.match(/\bversion\s*=\s*"([^"]+)"/);
  return inline?.[1] ?? null;
}

function exactVersion(spec) {
  const version = registryVersion(spec);
  const match = version?.match(/^=(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/);
  return match?.[1] ?? null;
}

export async function auditCargoDependencies(root) {
  const allowlist = JSON.parse(await readFile(path.join(root, 'docs/specs/security/rust-dependency-allowlist.json'), 'utf8')).packages ?? {};
  const errors = [];
  const seenRegistry = new Set();
  const manifests = await findManifests(root);
  for (const file of manifests) {
    const relative = path.relative(root, file);
    for (const { name, spec } of parseDependencies(await readFile(file, 'utf8'))) {
      if (/\bgit\s*=/.test(spec)) {
        errors.push(`${relative}: ${name} git dependencies are forbidden`);
        continue;
      }
      if (/\bpath\s*=/.test(spec) && !registryVersion(spec)) continue;
      if (/\bworkspace\s*=\s*true/.test(spec)) continue;
      const version = exactVersion(spec);
      if (!version) {
        errors.push(`${relative}: ${name} must use an exact version like =1.2.3`);
        continue;
      }
      seenRegistry.add(name);
      const approved = allowlist[name];
      if (!approved) {
        errors.push(`${relative}: ${name} is not in the Rust dependency allowlist`);
        continue;
      }
      if (approved.version !== version) errors.push(`${relative}: ${name} uses ${version}; allowlist approves ${approved.version}`);
      errors.push(...approvalErrors(name, approved));
    }
  }
  for (const name of Object.keys(allowlist)) {
    if (!seenRegistry.has(name)) errors.push(`${name} is allowlisted but not used by a Cargo manifest`);
  }
  return errors;
}

async function main() {
  const root = path.resolve(process.argv[2] ?? '.');
  const errors = await auditCargoDependencies(root);
  if (errors.length) {
    console.error(errors.map((error) => `FAIL ${error}`).join('\n'));
    process.exitCode = 1;
  } else {
    console.log('Cargo dependency metadata gate passed.');
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
