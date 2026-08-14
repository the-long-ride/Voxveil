import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { walkFilesSync } from '../lib/walk-files.mjs';

const IGNORED_DIRS = new Set(['.git', 'node_modules', 'dist', 'target', 'coverage']);
const FORBIDDEN_SUFFIXES = ['.tsbuildinfo', '.profraw', '.lcov'];
const FORBIDDEN_NAMES = new Set(['.DS_Store']);

export function scanRepositoryHygiene(root) {
  return walkFilesSync(root, IGNORED_DIRS)
    .filter((file) => FORBIDDEN_NAMES.has(path.basename(file)) || FORBIDDEN_SUFFIXES.some((suffix) => file.endsWith(suffix)))
    .map((file) => `${file} is generated build state and must not be committed`);
}

function main() {
  const issues = scanRepositoryHygiene(process.argv[2] ?? '.');
  if (issues.length === 0) {
    console.log('Repository hygiene gate passed.');
    return;
  }
  issues.forEach((issue) => console.error(`FAIL ${issue}`));
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
