import assert from 'node:assert/strict';
import { test } from 'node:test';
import { inspectCapability, inspectTauriConfig } from './check-tauri-security.mjs';

test('accepts the minimal core capability', () => {
  assert.deepEqual(inspectCapability({ permissions: ['core:default'] }, 'default.json'), []);
});

test('rejects broad shell filesystem and http capabilities', () => {
  const issues = inspectCapability({ permissions: ['shell:default', 'fs:read-all', 'http:default'] }, 'bad.json');
  assert.equal(issues.length, 3);
});

test('rejects remote IPC and remote production CSP sources', () => {
  const config = {
    app: {
      security: {
        csp: "default-src 'self' https://example.invalid",
        dangerousRemoteDomainIpcAccess: [{ domain: 'example.invalid' }],
      },
      withGlobalTauri: true,
    },
  };
  const issues = inspectTauriConfig(config);
  assert.match(issues.join('\n'), /remote CSP/i);
  assert.match(issues.join('\n'), /remote domain IPC/i);
  assert.match(issues.join('\n'), /global Tauri/i);
});
