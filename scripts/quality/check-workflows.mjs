import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REQUIRED = ['ci.yml', 'manual-build.yml', 'release.yml', 'windows-portable.yml'];

async function readWorkflow(root, file, errors) {
  try {
    return await readFile(path.join(root, '.github/workflows', file), 'utf8');
  } catch {
    errors.push(`${file} is missing`);
    return '';
  }
}

function requireText(file, text, required, errors) {
  for (const item of required) {
    if (!text.includes(item)) errors.push(`${file} must contain ${item}`);
  }
}

function auditActionPins(file, text, errors) {
  for (const match of text.matchAll(/uses:\s*([^\s#]+)/g)) {
    const reference = match[1];
    if (reference.startsWith('./')) continue;
    const separator = reference.lastIndexOf('@');
    const revision = separator >= 0 ? reference.slice(separator + 1) : '';
    if (!/^[0-9a-f]{40}$/i.test(revision)) {
      errors.push(`${file}: ${reference} must be pinned to an immutable 40-character SHA`);
    }
  }
}

function auditShellPipes(file, text, errors) {
  if (/(?:curl|wget)[^\n|]*\|\s*(?:sudo\s+)?(?:sh|bash)\b/i.test(text)) {
    errors.push(`${file}: pipe-to-shell installers are forbidden`);
  }
}

export async function auditWorkflows(root) {
  const errors = [];
  const workflows = Object.fromEntries(
    await Promise.all(REQUIRED.map(async (file) => [file, await readWorkflow(root, file, errors)])),
  );

  for (const [file, text] of Object.entries(workflows)) {
    auditActionPins(file, text, errors);
    auditShellPipes(file, text, errors);
  }

  requireText('ci.yml', workflows['ci.yml'], [
    'push:', 'pull_request:', 'master', 'contents: read', 'node scripts/ci/require-lockfiles.mjs', 'npm ci --ignore-scripts', 'npm run quality',
    'npm run coverage', 'cargo llvm-cov', '--fail-under-lines 85', 'cargo deny check',
  ], errors);
  requireText('manual-build.yml', workflows['manual-build.yml'], [
    'workflow_dispatch:', 'ref:', 'platform:', 'edition:', 'windows', 'linux', 'macos',
    'android', 'ios', 'standard', 'pro-system', 'node scripts/ci/require-lockfiles.mjs',
    'npm ci --ignore-scripts', 'npm run quality', 'npm run coverage', 'cargo llvm-cov',
    '--fail-under-lines 85', 'cargo deny check', 'actions/upload-artifact@',
  ], errors);
  requireText('release.yml', workflows['release.yml'], [
    'push:', 'tags:', 'v*.*.*', 'windows', 'linux', 'macos', 'android', 'ios',
    'standard', 'pro-system', 'node scripts/ci/require-lockfiles.mjs', 'npm ci --ignore-scripts',
    'npm run quality', 'npm run coverage', 'cargo llvm-cov', '--fail-under-lines 85',
    'cargo deny check', 'node scripts/release/verify-version.mjs',
    'node scripts/release/generate-release-metadata.mjs', 'node scripts/release/prepare-release-assets.mjs',
    'actions/upload-artifact@',
  ], errors);
  requireText('windows-portable.yml', workflows['windows-portable.yml'], [
    'push:', 'branches: [master]', 'windows-2025-vs2026', 'contents: read',
    'npm ci --ignore-scripts', 'build --no-bundle', 'target\\release\\voxveil.exe',
    'actions/upload-artifact@', 'compression-level: 0',
  ], errors);
  return errors;
}

async function main() {
  const root = path.resolve(process.argv[2] ?? '.');
  const errors = await auditWorkflows(root);
  if (errors.length) {
    console.error(errors.map((error) => `FAIL ${error}`).join('\n'));
    process.exitCode = 1;
    return;
  }
  console.log('Workflow policy gate passed.');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
