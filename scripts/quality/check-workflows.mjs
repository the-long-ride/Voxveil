import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ALLOWED_WORKFLOW = 'manual-build.yml';
const PLATFORM_INPUTS = [
  ['windows', 'true'],
  ['linux', 'false'],
  ['macos', 'false'],
];

function workflowDispatchOnly(content) {
  const onBlock = content.match(/^on:\s*\n((?:[ \t].*(?:\n|$))*)/m)?.[1] ?? '';
  const triggers = [...onBlock.matchAll(/^  ([A-Za-z0-9_-]+):/gm)].map((match) => match[1]);
  return triggers.length === 1 && triggers[0] === 'workflow_dispatch';
}

function hasBooleanInput(content, name, expectedDefault) {
  const inputBlock = content.match(
    new RegExp(`^      ${name}:\\s*\\n((?:        .*(?:\\n|$))*)`, 'm'),
  )?.[1];
  if (!inputBlock) return false;

  const hasBooleanType = /^        type:\s*boolean\s*$/m.test(inputBlock);
  const hasExpectedDefault = new RegExp(
    `^        default:\\s*${expectedDefault}\\s*$`,
    'm',
  ).test(inputBlock);
  return hasBooleanType && hasExpectedDefault;
}

export async function auditWorkflows(root) {
  const directory = path.join(root, '.github', 'workflows');
  try {
    const entries = await readdir(directory, { withFileTypes: true });
    const workflows = entries
      .filter((entry) => entry.isFile() && /\.ya?ml$/i.test(entry.name))
      .sort((left, right) => left.name.localeCompare(right.name));

    const errors = [];
    for (const workflow of workflows) {
      if (workflow.name !== ALLOWED_WORKFLOW) {
        errors.push(`${workflow.name} is forbidden: only ${ALLOWED_WORKFLOW} is allowed`);
        continue;
      }

      const content = await readFile(path.join(directory, workflow.name), 'utf8');
      if (!workflowDispatchOnly(content)) {
        errors.push(`${ALLOWED_WORKFLOW} must be workflow_dispatch-only`);
        continue;
      }

      for (const [name, expectedDefault] of PLATFORM_INPUTS) {
        if (!hasBooleanInput(content, name, expectedDefault)) {
          errors.push(
            `${ALLOWED_WORKFLOW} must define ${name} as boolean with default ${expectedDefault}`,
          );
        }
      }
    }
    return errors;
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
  console.log('Manual-build workflow policy gate passed.');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
