import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const infPath = 'native/windows/driver/package/VoxveilVirtualAudio.inf';

function inf() {
  return readFileSync(infPath, 'utf8');
}

test('virtual driver INF uses only Voxveil production identities', () => {
  const text = inf();
  assert.match(text, /Root\\VoxveilVirtualAudio/i);
  assert.match(text, /Voxveil Virtual Audio/i);
  assert.match(text, /Voxveil Input/i);
  assert.match(text, /CatalogFile\s*=\s*VoxveilVirtualAudio\.cat/i);
  assert.doesNotMatch(text, /Sysvad_|Tablet Audio Sample|Contoso|SwapAPO|DelayAPO/i);
});

test('virtual driver INF supports x64 and Arm64 without test-signing material', () => {
  const text = inf();
  assert.match(text, /\[Voxveil\.NTamd64(?:\.10\.0\.\.\.22621)?\]/i);
  assert.match(text, /\[Voxveil\.NTarm64(?:\.10\.0\.\.\.22621)?\]/i);
  assert.doesNotMatch(text, /\.cer\b|\.pfx\b|testsign|Test Certificate/i);
});

test('virtual driver INF preserves the pinned SysVAD Windows 11 22H2 applicability floor', () => {
  const text = inf();
  assert.match(text, /%MfgName%=Voxveil,NTamd64\.10\.0\.\.\.22621,NTarm64\.10\.0\.\.\.22621/i);
  assert.match(text, /\[Voxveil\.NTamd64\.10\.0\.\.\.22621\]/i);
  assert.match(text, /\[Voxveil\.NTarm64\.10\.0\.\.\.22621\]/i);
  assert.doesNotMatch(text, /\[Voxveil\.NT(?:amd64|arm64)\]/i);
});

test('virtual driver INF installs only the Voxveil driver binary', () => {
  const text = inf();
  assert.match(text, /VoxveilVirtualAudio\.sys/i);
  assert.doesNotMatch(text, /\.dll\b|devcon\.exe/i);
});

test('virtual driver validator rejects a SYS for the wrong architecture', () => {
  const text = readFileSync('scripts/windows/validate-virtual-driver-package.ps1', 'utf8');
  assert.match(text, /function\s+Get-PeMachine/i);
  assert.match(text, /0x8664/i);
  assert.match(text, /0xAA64/i);
  assert.match(text, /expectedMachine/i);
  assert.match(text, /driver architecture/i);
});

test('virtual driver validator finds WDK tools from both bin and Tools trees', () => {
  const text = readFileSync('scripts/windows/validate-virtual-driver-package.ps1', 'utf8');
  assert.match(text, /Windows Kits\\10\\bin/i);
  assert.match(text, /Windows Kits\\10\\Tools/i);
  assert.match(text, /InfVerif\.exe/i);
});

test('signed and unsigned package validation preserves the 22621 applicability floor', () => {
  const text = readFileSync('scripts/windows/validate-virtual-driver-package.ps1', 'utf8');
  assert.match(text, /22621/);
  assert.match(text, /driver applicability/i);
  assert.match(text, /NTamd64/i);
  assert.match(text, /NTarm64/i);
});

test('returned Microsoft package verification trusts the catalog and verifies its members', () => {
  const text = readFileSync('scripts/windows/validate-virtual-driver-package.ps1', 'utf8');
  assert.match(text, /verify\s+\/kp\s+\/v\s+\$cat\[0\]\.FullName/i);
  assert.match(text, /verify\s+\/c\s+\$cat\[0\]\.FullName\s+\/v\s+\$covered\.FullName/i);
  assert.doesNotMatch(text, /verify\s+\/kp\s+\/v\s+\$sys\[0\]\.FullName/i);
});

test('signed package verifier and stager use valid explicit cardinality checks', () => {
  for (const path of [
    'scripts/windows/verify-signed-virtual-driver.ps1',
    'scripts/windows/stage-signed-virtual-driver.ps1',
  ]) {
    const text = readFileSync(path, 'utf8');
    assert.doesNotMatch(text, /Select-Object\s+-Single/i, `${path} uses unsupported Select-Object -Single`);
    assert.match(text, /\.Count\s+-ne\s+1/, `${path} must reject ambiguous package members`);
  }
});

test('attestation CAB helper safely resolves makecab and quotes DDF output paths', () => {
  const text = readFileSync('scripts/windows/new-driver-attestation-cab.ps1', 'utf8');
  assert.match(text, /\$makecabCommand\s*=\s*Get-Command\s+makecab\.exe/i);
  assert.match(text, /if\s*\(\$makecabCommand\)/i);
  assert.doesNotMatch(text, /\(Get-Command\s+makecab\.exe[^\r\n]*\)\.Source/i);
  assert.match(text, /CabinetNameTemplate=`"\$cabName`"/i);
  assert.match(text, /DiskDirectoryTemplate=`"\$outputDir`"/i);
});

test('driver build keeps attestation CAB creation explicit at the release boundary', () => {
  const build = readFileSync('scripts/windows/build-virtual-driver.ps1', 'utf8');
  const release = readFileSync('docs/release/windows-driver-signing.md', 'utf8');
  assert.doesNotMatch(build, /new-driver-attestation-cab\.ps1/i);
  assert.match(release, /new-driver-attestation-cab\.ps1/i);
  assert.match(release, /native\\windows\\driver\\out\\x64\\submission/i);
});

test('virtual driver pins a concrete KMDF version and rejects unresolved INF tokens', () => {
  const sourceInf = inf();
  const project = readFileSync('native/windows/driver/VoxveilVirtualAudio.vcxproj', 'utf8');
  const validator = readFileSync('scripts/windows/validate-virtual-driver-package.ps1', 'utf8');

  assert.doesNotMatch(sourceInf, /\$KMDFVERSION\$/i);
  assert.match(sourceInf, /KmdfLibraryVersion\s*=\s*1\.15/i);
  assert.match(project, /<KMDF_VERSION_MAJOR>1<\/KMDF_VERSION_MAJOR>/i);
  assert.match(project, /<KMDF_VERSION_MINOR>15<\/KMDF_VERSION_MINOR>/i);
  assert.match(validator, /unresolved WDK template token/i);
});
