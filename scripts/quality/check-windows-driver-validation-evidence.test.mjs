import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const script = readFileSync('scripts/evaluation/collect-windows-driver-validation-evidence.ps1', 'utf8');

test('signed-driver validation collector binds evidence to exact checkout and Microsoft-signed package', () => {
  assert.match(script, /git\.exe/i);
  assert.match(script, /rev-parse HEAD/i);
  assert.match(script, /Requested Voxveil commit[\s\S]*current checkout/i);
  assert.match(script, /verify-signed-virtual-driver\.ps1/i);
  assert.match(script, /Microsoft-signed virtual driver verification failed/i);
  assert.match(script, /catalogThumbprint/i);
  assert.match(script, /SubmissionManifest/i);
  assert.match(script, /submission-manifest/i);
  assert.match(script, /Returned Microsoft-signed package INF\/SYS do not match/i);
  assert.match(script, /windowsDriverSamplesRevision/i);
  assert.match(script, /sysvadTreeSha/i);
  assert.match(script, /submissionManifestSha256/i);
});

test('signed-driver validation collector fails closed on release machine requirements', () => {
  assert.match(script, /Windows build 22621 or later/i);
  assert.match(script, /native Windows architecture/i);
  assert.match(script, /Confirm-SecureBootUEFI/i);
  assert.match(script, /Secure Boot must be enabled/i);
  assert.match(script, /TESTSIGNING must be off/i);
  assert.match(script, /bcdedit\.exe/i);
});

test('signed-driver validation collector revalidates retail release evidence and stays local/private', () => {
  assert.match(script, /release-evidence\.json/i);
  assert.match(script, /whcp-hlk/i);
  assert.match(script, /microsoft-approved-retail/i);
  assert.match(script, /release-evidence hashes do not match/i);
  assert.match(script, /\.local-evaluation\\windows-driver/i);
  assert.match(script, /lifecycle\s*=\s*\[ordered\]@\{/i);
  assert.match(script, /status\s*=\s*'pending'/i);
  assert.match(script, /No machine serial number/i);
  assert.doesNotMatch(script, /Partner Center credential[^']*=/i);
  assert.doesNotMatch(script, /git\s+(?:add|commit|push)/i);
});
