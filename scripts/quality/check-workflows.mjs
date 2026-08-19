import { readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export async function auditWorkflows(root) {
  const directory = path.join(root, '.github', 'workflows');
  try {
    const entries = await readdir(directory, { withFileTypes: true });
    return entries
      .filter((entry) => entry.isFile() && /\.ya?ml$/i.test(entry.name))
      .map((entry) => `${entry.name} is forbidden: Voxveil uses workflow-free local/manual builds`);
  } catch (error) {
    if (error?.code === 'ENOENT') return [];
    return [`unable to inspect .github/workflows: ${error.message}`];
  }
}

async function main() {
  const root = path.resolve(process.argv[2] ?? '.');
  const errors = await auditWorkflows(root);
  if (errors.length) {
    console.error(errors.map((error) => `FAIL ${error}`).join('\n'));
    process.exitCode = 1;
    return;
  }
  console.log('Workflow-free repository policy gate passed.');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
