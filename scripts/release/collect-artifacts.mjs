import { createHash } from 'node:crypto';
import { copyFile, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { readNamedArgs } from '../lib/cli.mjs';
import { walkFiles } from '../lib/walk-files.mjs';

const SEARCH_ROOTS = [
  'target/release/bundle',
  'tauri/gen/android/app/build/outputs',
  'tauri/gen/apple/build',
];
const PACKAGE_PATTERN = /\.(?:msi|exe|deb|rpm|dmg|apk|aab|ipa|zip|AppImage|tar\.gz)$/i;

async function sha256(file) {
  return createHash('sha256').update(await readFile(file)).digest('hex');
}

async function addWindowsPortableExecutable(candidates, root, platform) {
  if (platform !== 'windows') return;
  const executable = path.join(root, 'target/release/voxveil.exe');
  try {
    const metadata = await stat(executable);
    if (metadata.isFile()) candidates.push(executable);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
}

export async function collectArtifacts(root, platform, edition) {
  const candidates = [];
  for (const relative of SEARCH_ROOTS) {
    const files = await walkFiles(path.join(root, relative), { ignoreMissing: true });
    candidates.push(...files.filter((file) => PACKAGE_PATTERN.test(path.basename(file))));
  }
  await addWindowsPortableExecutable(candidates, root, platform);
  if (candidates.length === 0) throw new Error(`No build artifacts found for ${platform}/${edition}`);

  const outputDir = path.join(root, 'dist-artifacts', `${platform}-${edition}`);
  await rm(outputDir, { recursive: true, force: true });
  await mkdir(outputDir, { recursive: true });
  const files = [];
  const usedNames = new Set();
  for (const source of candidates.sort()) {
    let name = path.basename(source);
    if (usedNames.has(name)) name = `${platform}-${edition}-${name}`;
    usedNames.add(name);
    const destination = path.join(outputDir, name);
    await copyFile(source, destination);
    const metadata = await stat(destination);
    files.push({ name, sha256: await sha256(destination), size: metadata.size });
  }
  const sums = files.map((file) => `${file.sha256}  ${file.name}`).join('\n') + '\n';
  await writeFile(path.join(outputDir, 'SHA256SUMS'), sums);
  await writeFile(path.join(outputDir, 'manifest.json'), JSON.stringify({
    schemaVersion: 1,
    product: 'Voxveil',
    platform,
    edition,
    files,
  }, null, 2) + '\n');
  return { outputDir, files };
}

async function main() {
  const { platform, edition } = readNamedArgs(process.argv.slice(2), ['platform', 'edition']);
  const result = await collectArtifacts(process.cwd(), platform, edition);
  console.log(`Collected ${result.files.length} package(s) in ${result.outputDir}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
