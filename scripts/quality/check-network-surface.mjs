import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { walkFilesSync } from '../lib/walk-files.mjs';

const FORBIDDEN_DEPS = new Set(['@tauri-apps/plugin-http']);
const REMOTE_SPECIFIER = /^(?:git\+|https?:)/i;
const SOURCE_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx', '.mjs', '.rs']);
const APPROVED_RUST_NETWORK_FILE = 'tauri/models/download.rs';
const IGNORED_DIRS = new Set(['.git', 'node_modules', 'dist', 'coverage', 'target']);

export function inspectPackageJson(pkg, filePath) {
  const issues = [];
  for (const section of ['dependencies', 'devDependencies', 'optionalDependencies']) {
    for (const [name, specifier] of Object.entries(pkg[section] ?? {})) {
      if (FORBIDDEN_DEPS.has(name)) issues.push(`${filePath}: forbidden dependency ${name}`);
      if (REMOTE_SPECIFIER.test(String(specifier))) issues.push(`${filePath}: remote dependency ${name}`);
    }
  }
  return issues;
}

export function inspectProductionText(text, filePath) {
  let checks = [];
  if (filePath.startsWith('ui/')) {
    if (/\.test\.[cm]?[jt]sx?$/.test(filePath) || filePath.includes('/test/')) return [];
    checks = [
      [/\bfetch\s*\(/, 'fetch()'],
      [/\bnavigator\.sendBeacon\s*\(/, 'navigator.sendBeacon()'],
      [/\bnew\s+WebSocket\s*\(/, 'WebSocket'],
      [/\bnew\s+XMLHttpRequest\s*\(/, 'XMLHttpRequest'],
      [/\bnew\s+EventSource\s*\(/, 'EventSource'],
    ];
  } else if ((filePath.startsWith('tauri/') || filePath.startsWith('crates/')) && filePath.endsWith('.rs')) {
    checks = [
      [/\b(?:std|core)::net::TcpStream\b/, 'TcpStream'],
      [/\b(?:std|core)::net::UdpSocket\b/, 'UdpSocket'],
      [/\b(?:std|core)::net::TcpListener\b/, 'TcpListener'],
      [/\b(?:std|core)::net::ToSocketAddrs\b/, 'ToSocketAddrs'],
    ];
    if (filePath !== APPROVED_RUST_NETWORK_FILE) {
      checks.push([/\b(?:minreq|reqwest|ureq)::/, 'HTTP client outside approved model downloader']);
    }
  }
  return checks
    .filter(([pattern]) => pattern.test(text))
    .map(([, label]) => `${filePath}: ${label} is not allowed`);
}

export function scanNetworkSurface(root) {
  const resolved = path.resolve(root);
  const issues = [];
  for (const relative of walkFilesSync(resolved, IGNORED_DIRS)) {
    const absolute = path.join(resolved, relative);
    if (path.basename(relative) === 'package.json') {
      issues.push(...inspectPackageJson(JSON.parse(fs.readFileSync(absolute, 'utf8')), relative));
    } else if (SOURCE_EXTENSIONS.has(path.extname(relative))) {
      issues.push(...inspectProductionText(fs.readFileSync(absolute, 'utf8'), relative));
    }
  }
  return issues.sort();
}

function main() {
  const issues = scanNetworkSurface(process.argv[2] ?? '.');
  if (issues.length === 0) {
    console.log('Network surface gate passed.');
    return;
  }
  issues.forEach((issue) => console.error(`FAIL ${issue}`));
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
