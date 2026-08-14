import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { auditCargoDependencies } from './check-cargo-dependencies.mjs';

async function fixture(manifest, packages) {
  const root = await mkdtemp(path.join(os.tmpdir(), 'voxveil-cargo-'));
  await mkdir(path.join(root, 'docs/specs/security'), { recursive: true });
  await writeFile(path.join(root, 'Cargo.toml'), manifest);
  await writeFile(path.join(root, 'docs/specs/security/rust-dependency-allowlist.json'), JSON.stringify({ schemaVersion: 1, packages }));
  return root;
}

test('accepts path dependencies and exact allowlisted registry versions', async () => {
  const root = await fixture(`[dependencies]\nlocal = { path = "crates/local" }\nserde = { version = "=1.0.229", features = ["derive"] }\n`, {
    serde: { version: '1.0.229', license: 'MIT OR Apache-2.0', commercialUse: true },
  });
  assert.deepEqual(await auditCargoDependencies(root), []);
});

test('rejects git and floating registry dependencies', async () => {
  const root = await fixture(`[dependencies]\na = { git = "https://example.invalid/a" }\nb = "1.2"\n`, {});
  const errors = (await auditCargoDependencies(root)).join('\n');
  assert.match(errors, /git dependencies are forbidden/i);
  assert.match(errors, /exact version/i);
});

test('audits workspace dependency declarations instead of trusting workspace=true', async () => {
  const root = await fixture(`[workspace.dependencies]\nunsafe = { git = "https://example.invalid/unsafe" }\n`, {});
  const errors = (await auditCargoDependencies(root)).join('\n');
  assert.match(errors, /unsafe.*git dependencies are forbidden/i);
});

test('audits target-specific dependency sections', async () => {
  const root = await fixture(`[target.'cfg(windows)'.dependencies]\nunsafe = "1.2"\n`, {});
  const errors = (await auditCargoDependencies(root)).join('\n');
  assert.match(errors, /unsafe.*exact version/i);
});
