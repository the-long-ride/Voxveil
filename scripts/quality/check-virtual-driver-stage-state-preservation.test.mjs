import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const stager = readFileSync('scripts/windows/stage-signed-virtual-driver.ps1', 'utf8');

test('restaging a signed virtual driver never deletes or rewrites lifecycle install state', () => {
  assert.match(stager, /\$preservedInstallStateName\s*=\s*'virtual-driver-install-state\.json'/i);
  assert.match(stager, /Get-ChildItem\s+\$destination\s+-Force/i);
  assert.match(stager, /\$_\.Name\s+-ine\s+\$preservedInstallStateName/i);
  assert.match(stager, /Remove-Item\s+-Recurse\s+-Force/i);
  assert.doesNotMatch(stager, /Remove-Item\s+\$destination\s+-Recurse\s+-Force/i);
  assert.doesNotMatch(stager, /ReadAllBytes\(\$existingInstallStatePath\)|WriteAllBytes\([^\n]*virtual-driver-install-state\.json/i);
});

test('signed virtual-driver restaging invalidates old verification without touching lifecycle state', () => {
  const cleanup = stager.indexOf('Get-ChildItem $destination -Force');
  const firstCopy = stager.indexOf('Copy-Item $file.FullName', cleanup);
  const manifestWrite = stager.indexOf("Set-Content (Join-Path $destination 'verification.json')", firstCopy);
  assert.ok(cleanup >= 0 && firstCopy > cleanup, 'old non-state staging artifacts must be cleaned before new copies');
  assert.ok(manifestWrite > firstCopy, 'new verification marker must be published only after staging/copy checks');
});

test('signed virtual-driver staging rejects overlapping source and destination trees', () => {
  assert.match(stager, /\$destination\s*-eq\s*\$package/i);
  assert.match(stager, /\$destination\.StartsWith\(\$package\s*\+\s*\[IO\.Path\]::DirectorySeparatorChar/i);
  assert.match(stager, /\$package\.StartsWith\(\$destination\s*\+\s*\[IO\.Path\]::DirectorySeparatorChar/i);
  assert.match(stager, /must not overlap the source package directory/i);
});
