import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const build = readFileSync('scripts/windows/build-virtual-driver.ps1', 'utf8');
const cab = readFileSync('scripts/windows/new-driver-attestation-cab.ps1', 'utf8');

test('unsigned driver submission carries an exact-commit provenance manifest', () => {
  assert.match(build, /submission-manifest\.json/i);
  assert.match(build, /git\.exe/i);
  assert.match(build, /rev-parse HEAD/i);
  assert.match(build, /schemaVersion\s*=\s*1/i);
  assert.match(build, /voxveilCommit\s*=\s*\$voxveilCommit/i);
  assert.match(build, /windowsDriverSamplesRevision\s*=\s*\$PinnedRevision/i);
  assert.match(build, /sysvadTreeSha\s*=\s*\$PinnedSysvadTree/i);
  for (const field of ['infSha256', 'catalogSha256', 'driverSha256', 'pdbSha256']) {
    assert.match(build, new RegExp(`\\b${field}\\s*=\\s*Get-Sha256`, 'i'));
  }
});

test('attestation CAB intentionally contains driver submission files but not local handoff metadata', () => {
  assert.match(cab, /VoxveilVirtualAudio\.inf/i);
  assert.match(cab, /VoxveilVirtualAudio\.sys/i);
  assert.match(cab, /VoxveilVirtualAudio\.cat/i);
  assert.match(cab, /VoxveilVirtualAudio\.pdb/i);
  assert.doesNotMatch(cab, /submission-manifest\.json/i);
});
