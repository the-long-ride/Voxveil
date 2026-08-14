import { access } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REQUIRED = ['package-lock.json', 'Cargo.lock'];

export async function missingLockfiles(root) {
  const missing = [];
  for (const file of REQUIRED) {
    try { await access(path.join(root, file)); } catch { missing.push(file); }
  }
  return missing;
}

async function main() {
  const missing = await missingLockfiles(process.cwd());
  if (missing.length) {
    console.error(`Missing committed lockfiles: ${missing.join(', ')}. Generate and audit them on a trusted networked machine before CI/release.`);
    process.exitCode = 1;
  } else {
    console.log('Required lockfiles are present.');
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
