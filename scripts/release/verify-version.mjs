import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

async function readJsonVersion(root, file) {
  const value = JSON.parse(await readFile(path.join(root, file), 'utf8'));
  return value.version ?? null;
}

async function readCargoWorkspaceVersion(root) {
  const text = await readFile(path.join(root, 'Cargo.toml'), 'utf8');
  const section = text.match(/\[workspace\.package\]([\s\S]*?)(?=\n\[|$)/)?.[1] ?? '';
  return section.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
}

export async function verifyProjectVersion(root, tag) {
  const expected = tag.startsWith('v') ? tag.slice(1) : tag;
  if (!/^\d+\.\d+\.\d+$/.test(expected)) return [`invalid release version: ${tag}`];

  const versions = [
    ['package.json', await readJsonVersion(root, 'package.json')],
    ['ui/package.json', await readJsonVersion(root, 'ui/package.json')],
    ['tauri/package.json', await readJsonVersion(root, 'tauri/package.json')],
    ['tauri/tauri.conf.json', await readJsonVersion(root, 'tauri/tauri.conf.json')],
    ['Cargo.toml [workspace.package]', await readCargoWorkspaceVersion(root)],
  ];
  return versions
    .filter(([, version]) => version !== expected)
    .map(([file, version]) => `${file} version ${version ?? 'missing'} does not match ${expected}`);
}

async function main() {
  const tag = process.argv[2];
  if (!tag) throw new Error('Usage: node scripts/release/verify-version.mjs vX.Y.Z [root]');
  const root = path.resolve(process.argv[3] ?? '.');
  const errors = await verifyProjectVersion(root, tag);
  if (errors.length) {
    console.error(errors.map((error) => `FAIL ${error}`).join('\n'));
    process.exitCode = 1;
    return;
  }
  console.log(`Release version ${tag} matches all project manifests.`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
