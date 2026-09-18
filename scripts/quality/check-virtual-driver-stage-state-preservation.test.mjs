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


test('signed virtual-driver restaging refuses unexpected destination entries before cleanup', () => {
  assert.match(stager, /allowedDestinationNames/i);
  assert.match(stager, /unexpectedDestinationEntries/i);
  for (const name of [
    'virtual-driver-install-state.json',
    'VoxveilVirtualAudio.inf',
    'VoxveilVirtualAudio.cat',
    'VoxveilVirtualAudio.sys',
    'verification.json',
    'release-evidence.json',
  ]) {
    assert.match(stager, new RegExp(name.replaceAll('.', '\\.')));
  }
  const unexpectedGuard = stager.indexOf('$unexpectedDestinationEntries');
  const cleanup = stager.indexOf('Get-ChildItem $destination -Force', unexpectedGuard + 1);
  assert.ok(unexpectedGuard >= 0 && cleanup > unexpectedGuard, 'unexpected entries must be rejected before destination cleanup');
  assert.match(stager.slice(unexpectedGuard, cleanup), /refusing destructive restaging/i);
});


test('signed virtual-driver staging uses an architecture-scoped dist root before destination cleanup', () => {
  assert.match(stager, /\$distRoot/i);
  assert.match(stager, /windows-arm64/i);
  assert.match(stager, /windows-x64/i);
  assert.match(stager, /destination must be below the architecture-specific repository dist tree/i);

  const safety = stager.indexOf('destination must be under the repository dist tree');
  const destinationCreate = stager.indexOf('New-Item -ItemType Directory -Force -Path $destination');
  const cleanup = stager.indexOf('Get-ChildItem $destination -Force', destinationCreate);
  assert.ok(safety >= 0 && destinationCreate > safety && cleanup > safety, 'dist boundary must be checked before destination mutation');
});


test('signed virtual-driver staging rejects junction or symlink destination ancestors before cleanup', () => {
  assert.match(stager, /function\s+Assert-NoReparsePointInPath/i);
  assert.match(stager, /FileAttributes\]::ReparsePoint/i);
  const check = stager.indexOf('Assert-NoReparsePointInPath');
  const mutation = stager.indexOf('New-Item -ItemType Directory -Force -Path $destination');
  assert.ok(check >= 0 && mutation > check, 'reparse-point preflight must precede staging mutation');
});
