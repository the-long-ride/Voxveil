import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const validation = readFileSync('scripts/evaluation/record-windows-apo-validation-evidence.ps1', 'utf8');
const qualification = readFileSync('scripts/evaluation/record-windows-apo-qualification-evidence.ps1', 'utf8');

test('APO validation recorder binds real-machine observations to the final signed package and secure boot state', () => {
  assert.match(validation, /release-manifest\.json/i);
  assert.match(validation, /apo-verification\.json/i);
  assert.match(validation, /signedApo\.present/i);
  assert.match(validation, /Confirm-SecureBootUEFI/i);
  assert.match(validation, /TESTSIGNING must be off/i);
  assert.match(validation, /LastBootUpTime/i);
  assert.match(validation, /releaseManifestSha256/i);
  assert.match(validation, /apoVerificationSha256/i);
  assert.match(validation, /supported-hardware-matrix/i);
  assert.match(validation, /reboot-resume evidence requires a changed Windows boot marker/i);
  assert.doesNotMatch(validation, /git\s+(?:add|commit|push)/i);
});

test('APO qualification recorder hashes externally retained Retail evidence without storing paths or credentials', () => {
  assert.match(qualification, /whcp-hlk/i);
  assert.match(qualification, /microsoft-approved-retail/i);
  assert.match(qualification, /releaseChannel[^\r\n]*retail/i);
  assert.match(qualification, /Get-Sha256/i);
  assert.match(qualification, /fileName/i);
  assert.match(qualification, /bytes/i);
  assert.match(qualification, /Only external evidence file names, sizes, and SHA-256/i);
  assert.doesNotMatch(qualification, /Partner Center credentials\s*=/i);
});
