import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const installer = readFileSync('scripts/windows/install-system-audio-component.ps1', 'utf8');
const uninstaller = readFileSync('scripts/windows/uninstall-system-audio-component.ps1', 'utf8');

test('development APO certificate trust is recorded as exact install-state ownership', () => {
  assert.match(installer, /New-SelfSignedCertificate/i);
  assert.match(installer, /CN=Voxveil Development APO/i);
  assert.match(installer, /developmentCertificateThumbprint/i);
  assert.match(installer, /certificate\.Thumbprint/i);
  assert.match(installer, /\^\[0-9A-Fa-f\]\{40\}\\z/i);
});

test('APO uninstaller removes only recorded Voxveil development certificate thumbprints', () => {
  assert.match(uninstaller, /function\s+Remove-RecordedDevelopmentCertificate/i);
  assert.match(uninstaller, /developmentCertificateThumbprint/i);
  for (const store of ['My', 'Root', 'TrustedPublisher']) {
    assert.match(uninstaller, new RegExp(store, 'i'));
  }
  assert.match(uninstaller, /CN=Voxveil Development APO/i);
  assert.match(uninstaller, /Remove-Item\s+\$certificatePath\s+-Force/i);
});

test('pre-ownership test certificate is cleaned if the first lifecycle snapshot fails', () => {
  assert.match(installer, /developmentCertificateOwnedByState/i);
  assert.match(installer, /function\s+Remove-RecordedDevelopmentCertificate/i);

  const certCreate = installer.indexOf('$certificate = New-SelfSignedCertificate');
  const snapshot = installer.indexOf('Write-InstallStateSnapshot', certCreate);
  const markOwned = installer.indexOf('$script:developmentCertificateOwnedByState = $true', snapshot);
  assert.ok(certCreate >= 0 && snapshot > certCreate, 'new certificate ownership must be snapshotted immediately');
  assert.ok(markOwned > snapshot, 'certificate must become state-owned only after the snapshot succeeds');

  const finallyStart = installer.indexOf('finally {');
  assert.ok(finallyStart >= 0, 'installer must have final cleanup');
  const finalCleanup = installer.slice(finallyStart);
  assert.match(finalCleanup, /-not\s+\$script:developmentCertificateOwnedByState/i);
  assert.match(finalCleanup, /Remove-RecordedDevelopmentCertificate\s+\$developmentCertificateThumbprint/i);
});
