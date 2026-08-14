import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const FORBIDDEN_PERMISSION_PREFIXES = ['shell:', 'fs:', 'http:'];

function permissionId(permission) {
  if (typeof permission === 'string') return permission;
  return permission?.identifier ?? permission?.id ?? '';
}

export function inspectCapability(capability, filePath) {
  const issues = [];
  for (const permission of capability.permissions ?? []) {
    const id = permissionId(permission);
    if (FORBIDDEN_PERMISSION_PREFIXES.some((prefix) => id.startsWith(prefix))) {
      issues.push(`${filePath}: forbidden broad capability ${id}`);
    }
  }
  return issues;
}

export function inspectTauriConfig(config) {
  const issues = [];
  const security = config.app?.security ?? {};
  const csp = typeof security.csp === 'string' ? security.csp : JSON.stringify(security.csp ?? '');
  if (/\b(?:https?|wss?):/i.test(csp)) issues.push('tauri.conf.json: remote CSP sources are forbidden');
  if ((security.dangerousRemoteDomainIpcAccess ?? []).length > 0) {
    issues.push('tauri.conf.json: remote domain IPC access is forbidden');
  }
  if (config.app?.withGlobalTauri === true) issues.push('tauri.conf.json: global Tauri injection is forbidden');
  return issues;
}

export async function auditTauriSecurity(root) {
  const issues = [];
  const config = JSON.parse(await readFile(path.join(root, 'tauri/tauri.conf.json'), 'utf8'));
  issues.push(...inspectTauriConfig(config));
  const capabilityDir = path.join(root, 'tauri/capabilities');
  for (const entry of await readdir(capabilityDir, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith('.json')) continue;
    const capability = JSON.parse(await readFile(path.join(capabilityDir, entry.name), 'utf8'));
    issues.push(...inspectCapability(capability, `tauri/capabilities/${entry.name}`));
  }
  return issues.sort();
}

async function main() {
  const root = path.resolve(process.argv[2] ?? '.');
  const issues = await auditTauriSecurity(root);
  if (issues.length) {
    console.error(issues.map((issue) => `FAIL ${issue}`).join('\n'));
    process.exitCode = 1;
  } else {
    console.log('Tauri security capability gate passed.');
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
