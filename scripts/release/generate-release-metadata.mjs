import { execFile } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';

const execFileAsync = promisify(execFile);

function componentLicense(license) {
  if (!license) return undefined;
  if (/^[A-Za-z0-9-.+]+$/.test(license)) return [{ license: { id: license } }];
  return [{ expression: license }];
}

function npmName(lockPath, entry) {
  if (entry.name) return entry.name;
  const marker = 'node_modules/';
  const index = lockPath.lastIndexOf(marker);
  return index >= 0 ? lockPath.slice(index + marker.length) : null;
}

function baseSbom(timestamp, component) {
  return {
    bomFormat: 'CycloneDX',
    specVersion: '1.6',
    version: 1,
    metadata: { timestamp, component },
    components: [],
  };
}

export function packageLockToCycloneDx(lock, timestamp = new Date().toISOString()) {
  const root = {
    type: 'application',
    name: lock.name ?? 'voxveil',
    version: lock.version ?? '0.0.0',
  };
  const sbom = baseSbom(timestamp, root);
  for (const [lockPath, entry] of Object.entries(lock.packages ?? {})) {
    if (!lockPath.includes('node_modules/') || !entry?.version) continue;
    const name = npmName(lockPath, entry);
    if (!name) continue;
    const component = {
      type: 'library',
      name,
      version: entry.version,
      purl: `pkg:npm/${encodeURIComponent(name).replace('%2F', '/')}@${entry.version}`,
    };
    const licenses = componentLicense(entry.license);
    if (licenses) component.licenses = licenses;
    sbom.components.push(component);
  }
  sbom.components.sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
  return sbom;
}

export function cargoMetadataToCycloneDx(metadata, timestamp = new Date().toISOString()) {
  const sbom = baseSbom(timestamp, { type: 'application', name: 'voxveil-rust-workspace' });
  for (const pkg of metadata.packages ?? []) {
    if (!pkg.source?.startsWith('registry+')) continue;
    const component = {
      type: 'library',
      name: pkg.name,
      version: pkg.version,
      purl: `pkg:cargo/${pkg.name}@${pkg.version}`,
    };
    const licenses = componentLicense(pkg.license);
    if (licenses) component.licenses = licenses;
    sbom.components.push(component);
  }
  sbom.components.sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
  return sbom;
}

async function readJson(file) {
  return JSON.parse(await readFile(file, 'utf8'));
}

async function main() {
  const root = path.resolve(process.argv[2] ?? '.');
  const output = path.join(root, 'release-metadata');
  const timestamp = new Date().toISOString();
  const packageLock = await readJson(path.join(root, 'package-lock.json'));
  const { stdout } = await execFileAsync('cargo', ['metadata', '--locked', '--format-version=1'], {
    cwd: root,
    maxBuffer: 32 * 1024 * 1024,
  });
  const cargoMetadata = JSON.parse(stdout);
  const npmSbom = packageLockToCycloneDx(packageLock, timestamp);
  const rustSbom = cargoMetadataToCycloneDx(cargoMetadata, timestamp);
  const npmPolicy = await readJson(path.join(root, 'docs/specs/security/dependency-allowlist.json'));
  const rustPolicy = await readJson(path.join(root, 'docs/specs/security/rust-dependency-allowlist.json'));

  await mkdir(output, { recursive: true });
  await Promise.all([
    writeFile(path.join(output, 'npm-sbom.cdx.json'), `${JSON.stringify(npmSbom, null, 2)}\n`),
    writeFile(path.join(output, 'rust-sbom.cdx.json'), `${JSON.stringify(rustSbom, null, 2)}\n`),
    writeFile(path.join(output, 'approved-npm-dependencies.json'), `${JSON.stringify(npmPolicy, null, 2)}\n`),
    writeFile(path.join(output, 'approved-rust-dependencies.json'), `${JSON.stringify(rustPolicy, null, 2)}\n`),
    writeFile(path.join(output, 'manifest.json'), `${JSON.stringify({
      generatedAt: timestamp,
      files: [
        'npm-sbom.cdx.json',
        'rust-sbom.cdx.json',
        'approved-npm-dependencies.json',
        'approved-rust-dependencies.json',
      ],
    }, null, 2)}\n`),
  ]);
  console.log(`Release metadata written to ${path.relative(root, output)}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
