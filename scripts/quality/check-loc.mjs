import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { walkFilesSync } from '../lib/walk-files.mjs';

const LIMITS = new Map([
  ['.ts', 300],
  ['.tsx', 400],
  ['.rs', 300],
  ['.css', 400],
]);

const IGNORED_DIRS = new Set(['.git', 'node_modules', 'dist', 'target', 'coverage']);

export function limitFor(filePath) {
  return LIMITS.get(path.extname(filePath)) ?? null;
}

export function countPhysicalLines(text) {
  if (text.length === 0) return 0;
  const count = text.split('\n').length;
  return text.endsWith('\n') ? count - 1 : count;
}

export function violationFor(filePath, text) {
  const limit = limitFor(filePath);
  if (limit === null) return null;
  const lines = countPhysicalLines(text);
  return lines > limit ? { path: filePath, lines, limit } : null;
}

export function scan(root) {
  const resolved = path.resolve(root);
  const violations = [];
  for (const relative of walkFilesSync(resolved, IGNORED_DIRS)) {
    if (limitFor(relative) === null) continue;
    const violation = violationFor(relative, fs.readFileSync(path.join(resolved, relative), 'utf8'));
    if (violation) violations.push(violation);
  }
  return violations;
}

function main() {
  const root = process.argv[2] ?? '.';
  const violations = scan(root);
  if (violations.length === 0) {
    console.log('LOC gate passed.');
    return;
  }
  for (const item of violations) {
    console.error(`FAIL ${item.path}: ${item.lines}/${item.limit} lines`);
  }
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
