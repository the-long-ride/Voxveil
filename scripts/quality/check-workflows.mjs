import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ALLOWED_WORKFLOW = 'manual-build.yml';
const VERIFICATION_PUSH_BRANCH = 'feat/windows-signed-audio-paths';
const PLATFORM_INPUTS = [
  ['windows', 'true'],
  ['linux', 'false'],
  ['macos', 'false'],
];

function workflowOnBlock(content) {
  const lines = content.replaceAll('\r\n', '\n').split('\n');
  const onIndex = lines.findIndex((line) => line.trimEnd() === 'on:');
  if (onIndex < 0) return [];

  const block = [];
  for (let index = onIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === '') {
      if (block.length) break;
      continue;
    }
    if (!line.startsWith(' ')) break;
    block.push(line);
  }
  return block;
}

function workflowTriggers(content) {
  return workflowOnBlock(content)
    .filter((line) => /^  [A-Za-z0-9_-]+:\s*$/.test(line))
    .map((line) => line.trim().slice(0, -1));
}

function exactVerificationPush(content) {
  const block = workflowOnBlock(content);
  const pushIndex = block.findIndex((line) => line.trim() === 'push:');
  if (pushIndex < 0) return false;

  const pushLines = [];
  for (let index = pushIndex + 1; index < block.length; index += 1) {
    const line = block[index];
    if (/^  [A-Za-z0-9_-]+:\s*$/.test(line)) break;
    pushLines.push(line.trim());
  }
  const meaningful = pushLines.filter(Boolean);
  return (
    meaningful.length === 2 &&
    meaningful[0] === 'branches:' &&
    meaningful[1] === '- ' + VERIFICATION_PUSH_BRANCH
  );
}

function workflowTriggersAllowed(content) {
  const triggers = workflowTriggers(content);
  if (triggers.length === 1) {
    return triggers[0] === 'workflow_dispatch';
  }
  return (
    triggers.length === 2 &&
    triggers.includes('workflow_dispatch') &&
    triggers.includes('push') &&
    exactVerificationPush(content)
  );
}

function hasBooleanInput(content, name, expectedDefault) {
  const normalized = content.replaceAll('\r\n', '\n');
  const inputBlock = normalized.match(
    new RegExp(`^      ${name}:\\s*\\n((?:        .*(?:\\n|$))*)`, 'm'),
  )?.[1];
  if (!inputBlock) return false;

  const hasBooleanType = /^        type:\s*boolean\s*$/m.test(inputBlock);
  const hasExpectedDefault = new RegExp(
    `^        default:\\s*${expectedDefault}\\s*import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ALLOWED_WORKFLOW = 'manual-build.yml';
const VERIFICATION_PUSH_BRANCH = 'feat/windows-signed-audio-paths';
const PLATFORM_INPUTS = [
  ['windows', 'true'],
  ['linux', 'false'],
  ['macos', 'false'],
];

function workflowOnBlock(content) {
  const lines = content.replaceAll('\r\n', '\n').split('\n');
  const onIndex = lines.findIndex((line) => line.trimEnd() === 'on:');
  if (onIndex < 0) return [];

  const block = [];
  for (let index = onIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === '') {
      if (block.length) break;
      continue;
    }
    if (!line.startsWith(' ')) break;
    block.push(line);
  }
  return block;
}

function workflowTriggers(content) {
  return workflowOnBlock(content)
    .filter((line) => /^  [A-Za-z0-9_-]+:\s*$/.test(line))
    .map((line) => line.trim().slice(0, -1));
}

function exactVerificationPush(content) {
  const block = workflowOnBlock(content);
  const pushIndex = block.findIndex((line) => line.trim() === 'push:');
  if (pushIndex < 0) return false;

  const pushLines = [];
  for (let index = pushIndex + 1; index < block.length; index += 1) {
    const line = block[index];
    if (/^  [A-Za-z0-9_-]+:\s*$/.test(line)) break;
    pushLines.push(line.trim());
  }
  const meaningful = pushLines.filter(Boolean);
  return (
    meaningful.length === 2 &&
    meaningful[0] === 'branches:' &&
    meaningful[1] === '- ' + VERIFICATION_PUSH_BRANCH
  );
}

function workflowTriggersAllowed(content) {
  const triggers = workflowTriggers(content);
  if (triggers.length === 1) {
    return triggers[0] === 'workflow_dispatch';
  }
  return (
    triggers.length === 2 &&
    triggers.includes('workflow_dispatch') &&
    triggers.includes('push') &&
    exactVerificationPush(content)
  );
}

,
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
      if (!workflowTriggersAllowed(content)) {
        errors.push(
          ALLOWED_WORKFLOW +
            ' must be workflow_dispatch-only except for the exact ' +
            VERIFICATION_PUSH_BRANCH +
            ' verification push',
        );
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
