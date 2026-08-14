import { spawnSync } from 'node:child_process';
import { readdir } from 'node:fs/promises';
import path from 'node:path';

async function findTests(dir, output = []) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) await findTests(full, output);
    else if (entry.isFile() && entry.name.endsWith('.test.mjs')) output.push(full);
  }
  return output;
}

const tests = (await findTests(path.resolve('scripts'))).sort();
const result = spawnSync(process.execPath, ['--test', ...tests], { stdio: 'inherit' });
process.exit(result.status ?? 1);
