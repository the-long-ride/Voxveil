import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { walkFilesSync } from '../lib/walk-files.mjs';

const MODEL_EXTENSIONS = new Set(['.onnx', '.pt', '.pth', '.ckpt', '.safetensors', '.tflite', '.mlmodel']);
const SHA256 = /^[a-f0-9]{64}$/;
const COMMIT_SHA = /^[a-f0-9]{40}$/;
const IGNORED_DIRS = new Set(['.git', 'node_modules', 'dist', 'coverage', 'target']);

export function inspectCatalog(entries) {
  const issues = [];
  const ids = new Set();
  for (const entry of entries) {
    if (!entry.id || ids.has(entry.id)) issues.push(`${entry.id || '<missing>'}: model id must be unique`);
    ids.add(entry.id);
    if (!String(entry.downloadUrl ?? '').startsWith('https://')) issues.push(`${entry.id}: download URL must use HTTPS`);
    const revision = String(entry.sourceRevision ?? '');
    const downloadUrl = String(entry.downloadUrl ?? '');
    if (!COMMIT_SHA.test(revision)) issues.push(`${entry.id}: source revision must be a 40-character commit SHA`);
    if (revision && !downloadUrl.includes(`/resolve/${revision}/`)) issues.push(`${entry.id}: download URL must pin the declared source revision`);
    if (!SHA256.test(String(entry.sha256 ?? ''))) issues.push(`${entry.id}: exact SHA-256 is required`);
    if (entry.commercialUse !== true) issues.push(`${entry.id}: commercial use must be explicitly approved`);
    if (entry.directDownloadOnly !== true) issues.push(`${entry.id}: model must be downloaded directly by the user`);
    if (!entry.license || !entry.fileName || !Number.isInteger(entry.maxBytes) || entry.maxBytes <= 0) {
      issues.push(`${entry.id}: complete license, filename and size-limit metadata are required`);
    }
  }
  return issues;
}

export function inspectModelFiles(repositoryFiles, bundleResources) {
  const issues = [];
  for (const file of repositoryFiles) {
    if (MODEL_EXTENSIONS.has(path.extname(file).toLowerCase())) issues.push(`${file}: model binary must not be committed or bundled`);
  }
  for (const resource of bundleResources) {
    if (MODEL_EXTENSIONS.has(path.extname(String(resource)).toLowerCase())) issues.push(`${resource}: model binary is forbidden as a bundle resource`);
  }
  return issues;
}

export function auditModelDelivery(root) {
  const resolved = path.resolve(root);
  const catalogPath = path.join(resolved, 'tauri/models/catalog.json');
  const catalog = JSON.parse(fs.readFileSync(catalogPath, 'utf8'));
  const repositoryFiles = walkFilesSync(resolved, IGNORED_DIRS);
  const tauriConfig = JSON.parse(fs.readFileSync(path.join(resolved, 'tauri/tauri.conf.json'), 'utf8'));
  const resources = tauriConfig.bundle?.resources ?? [];
  return [...inspectCatalog(catalog.models ?? []), ...inspectModelFiles(repositoryFiles, resources)].sort();
}

function main() {
  const issues = auditModelDelivery(process.argv[2] ?? '.');
  if (!issues.length) {
    console.log('AI model delivery gate passed.');
    return;
  }
  issues.forEach((issue) => console.error(`FAIL ${issue}`));
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
