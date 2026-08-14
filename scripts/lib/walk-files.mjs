import fs from 'node:fs';
import { readdir } from 'node:fs/promises';
import path from 'node:path';

export function walkFilesSync(root, ignoredDirs = new Set()) {
  const resolved = path.resolve(root);
  const files = [];

  function walk(current) {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      if (entry.isDirectory() && ignoredDirs.has(entry.name)) continue;
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) walk(absolute);
      else if (entry.isFile()) files.push(path.relative(resolved, absolute).replaceAll(path.sep, '/'));
    }
  }

  walk(resolved);
  return files.sort();
}

export async function walkFiles(root, { ignoreMissing = false } = {}) {
  const resolved = path.resolve(root);
  const files = [];

  async function walk(current) {
    let entries;
    try {
      entries = await readdir(current, { withFileTypes: true });
    } catch (error) {
      if (ignoreMissing && error?.code === 'ENOENT') return;
      throw error;
    }
    for (const entry of entries) {
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) await walk(absolute);
      else if (entry.isFile()) files.push(absolute);
    }
  }

  await walk(resolved);
  return files.sort();
}
