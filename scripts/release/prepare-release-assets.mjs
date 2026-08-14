import { copyFile, mkdir, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { walkFiles } from '../lib/walk-files.mjs';

function releaseName(inputDir, file) {
  return path.relative(inputDir, file).split(path.sep).join('--');
}

export async function prepareReleaseAssets(inputDir, outputDir) {
  const files = await walkFiles(inputDir);
  if (files.length === 0) throw new Error('No downloaded workflow artifacts found');

  await rm(outputDir, { recursive: true, force: true });
  await mkdir(outputDir, { recursive: true });
  const assets = [];
  for (const source of files) {
    const name = releaseName(inputDir, source);
    await copyFile(source, path.join(outputDir, name));
    assets.push(name);
  }
  return assets;
}

async function main() {
  const input = path.resolve(process.argv[2] ?? 'release-artifacts');
  const output = path.resolve(process.argv[3] ?? 'release-assets');
  const assets = await prepareReleaseAssets(input, output);
  console.log(`Prepared ${assets.length} unique release assets in ${output}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
