import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

const expectedFiles = [
  'VoxveilApo.inf',
  'VoxveilApo.dll',
  'VoxveilApo.cat',
  'VoxveilApoExtension.inf',
  'VoxveilApoExtension.cat',
];

test('signed APO verifier checks exact package identities and catalog coverage', () => {
  const text = read('scripts/windows/verify-signed-apo-package.ps1');
  for (const name of expectedFiles) assert.match(text, new RegExp(name.replace('.', '\\.')));
  assert.match(text, /F3F2A99F-8FB7-4B88-949E-448BF8A05221/i);
  assert.match(text, /63E268CE-4CBC-48E0-BEB6-55103316F477/i);
  assert.match(text, /1D81E93D-AB81-473B-9E5E-94FAE8D2377F/i);
  assert.match(text, /verify\s+\/kp\s+\/v/i);
  assert.match(text, /verify\s+\/c/i);
  assert.match(text, /verify\s+\/pa\s+\/v\s+\$apoDll\.FullName/i);
  assert.match(text, /SignatureAttributes\.PETrust/i);
  assert.match(text, /Get-PeMachine/i);
  assert.match(text, /0x8664/i);
  assert.match(text, /InfVerif\.exe/i);
  assert.doesNotMatch(text, /TESTSIGNING|New-SelfSignedCertificate|signtool\s+sign/i);
});

test('signed APO stager copies only the verified production package', () => {
  const text = read('scripts/windows/stage-signed-apo-package.ps1');
  assert.match(text, /verify-signed-apo-package\.ps1/i);
  for (const name of expectedFiles) assert.match(text, new RegExp(name.replace('.', '\\.')));
  assert.match(text, /apo-verification\.json/i);
  assert.match(text, /apoInfSha256/);
  assert.match(text, /apoDllSha256/);
  assert.match(text, /apoCatalogSha256/);
  assert.match(text, /extensionInfSha256/);
  assert.match(text, /extensionCatalogSha256/);
  assert.doesNotMatch(text, /VoxveilDevelopment|\.cer\b|\.pfx\b/i);
});

test('signed APO stager rehashes destination copies before writing verification manifest', () => {
  const text = read('scripts/windows/stage-signed-apo-package.ps1');
  assert.match(text, /function\s+Assert-StagedHash[\s\S]*?Get-FileHash/i);

  const copy = text.search(/Copy-Item/i);
  const firstHash = text.indexOf('Assert-StagedHash -Path $stagedApoInf', copy);
  const manifestWrite = text.search(/Set-Content\s+\$manifestPath\s+-Encoding\s+utf8/i);
  assert.ok(copy >= 0, 'stager must copy the verified package');
  assert.ok(firstHash > copy, 'destination hashes must be checked after copying');
  assert.ok(manifestWrite > firstHash, 'verification manifest must be written only after destination hashes pass');

  for (const [pathVar, hashField] of [
    ['$stagedApoInf', 'apoInfSha256'],
    ['$stagedApoDll', 'apoDllSha256'],
    ['$stagedApoCat', 'apoCatalogSha256'],
    ['$stagedExtensionInf', 'extensionInfSha256'],
    ['$stagedExtensionCat', 'extensionCatalogSha256'],
  ]) {
    assert.match(
      text,
      new RegExp(`Assert-StagedHash\\s+-Path\\s+\\${pathVar.replace('$', '$')}\\s+-Expected\\s+\\$verification\\.${hashField}`, 'i'),
    );
  }
});

test('signed APO restaging invalidates any old verification manifest before replacing artifacts', () => {
  const text = read('scripts/windows/stage-signed-apo-package.ps1');
  assert.match(text, /\$manifestPath\s*=\s*Join-Path\s+\$destination\s+'apo-verification\.json'/i);
  const invalidate = text.search(/Remove-Item\s+\$manifestPath\s+-Force\s+-ErrorAction\s+SilentlyContinue/i);
  const copy = text.search(/Copy-Item/i);
  const manifestWrite = text.search(/Set-Content\s+\$manifestPath\s+-Encoding\s+utf8/i);
  assert.ok(invalidate >= 0, 'stager must invalidate any old verification marker before changing signed artifacts');
  assert.ok(copy > invalidate, 'old verification marker must be removed before any signed artifact is replaced');
  assert.ok(manifestWrite > copy, 'a new verification marker must be published only after staging work completes');
});

test('production endpoint discovery requires the verified APO staging marker', () => {
  const module = read('crates/voxveil-windows-audio/src/discovery.rs');
  const implementation = read('crates/voxveil-windows-audio/src/discovery_windows.rs');
  assert.match(module, /discovery_windows\.rs/);
  assert.match(implementation, /apo-verification\.json/i);
  assert.match(implementation, /1D81E93D-AB81-473B-9E5E-94FAE8D2377F/i);
  assert.match(implementation, /63E268CE-4CBC-48E0-BEB6-55103316F477/i);
});

test('production installer rehashes every signed APO artifact before installation', () => {
  const text = read('scripts/windows/install-system-audio-component.ps1');
  assert.match(text, /apo-verification\.json/i);
  assert.match(text, /Get-FileHash/i);
  for (const field of [
    'apoInfSha256',
    'apoDllSha256',
    'apoCatalogSha256',
    'extensionInfSha256',
    'extensionCatalogSha256',
  ]) {
    assert.match(text, new RegExp(field));
  }
});

test('Windows package build stages a signed APO only through the verifier path', () => {
  const text = read('scripts/windows/build-windows.ps1');
  assert.match(text, /VOXVEIL_SIGNED_APO_DIR/);
  assert.match(text, /stage-signed-apo-package\.ps1/i);
  assert.doesNotMatch(text, /Copy-Item\s+\$env:VOXVEIL_SIGNED_APO_DIR/i);
});


test('signed APO staging is confined below repository dist/windows-x64 before any destination mutation', () => {
  const text = read('scripts/windows/stage-signed-apo-package.ps1');
  assert.match(text, /\$repoRoot/i);
  assert.match(text, /\$distRoot/i);
  assert.match(text, /Test-DirectoryContains/i);
  assert.match(text, /destination must be below the repository dist\\windows-x64 tree/i);

  const safety = text.indexOf('destination must be below the repository dist\\windows-x64 tree');
  const manifestRemoval = text.indexOf('Remove-Item $manifestPath -Force -ErrorAction SilentlyContinue');
  const copy = text.indexOf('Copy-Item $file.FullName');
  assert.ok(safety >= 0 && manifestRemoval > safety && copy > safety, 'destination safety must run before mutation');
});

test('signed APO staging rejects source and destination overlap in either direction', () => {
  const text = read('scripts/windows/stage-signed-apo-package.ps1');
  assert.match(text, /Test-DirectoryOverlap/i);
  assert.match(text, /must not overlap the source package directory/i);
});


test('production APO installer revalidates staged Microsoft signer identities before PnP mutation', () => {
  const text = read('scripts/windows/install-system-audio-component.ps1');
  const preflight = text.indexOf('function Assert-StagedProductionApo');
  const firstPnp = text.indexOf("pnputil.exe /add-driver (Join-Path $work 'VoxveilApo.inf') /install");
  assert.ok(preflight >= 0 && firstPnp > preflight);

  const block = text.slice(preflight, firstPnp);
  assert.match(block, /function\s+Assert-StagedMicrosoftSigner/i);
  assert.match(block, /Get-AuthenticodeSignature/i);
  assert.match(block, /SignerCertificate\.Subject/i);
  assert.match(block, /Microsoft/i);
  assert.match(block, /apoSigner/i);
  assert.match(block, /apoCatalogSigner/i);
  assert.match(block, /extensionCatalogSigner/i);
  assert.match(block, /signer does not match apo-verification\.json/i);
});


test('signed APO provenance carries exact signer certificate thumbprints through install preflight', () => {
  const verifier = read('scripts/windows/verify-signed-apo-package.ps1');
  const stager = read('scripts/windows/stage-signed-apo-package.ps1');
  const installer = read('scripts/windows/install-system-audio-component.ps1');

  for (const field of ['apoThumbprint', 'apoCatalogThumbprint', 'extensionCatalogThumbprint']) {
    assert.match(verifier, new RegExp(field, 'i'));
    assert.match(stager, new RegExp(field, 'i'));
    assert.match(installer, new RegExp(field, 'i'));
  }
  assert.match(verifier, /SignerCertificate\.Thumbprint/i);
  assert.match(installer, /SignerCertificate\.Thumbprint/i);
  assert.match(installer, /signer thumbprint does not match apo-verification\.json/i);
});


test('signed APO staging rejects junction or symlink destination ancestors before mutation', () => {
  const text = read('scripts/windows/stage-signed-apo-package.ps1');
  assert.match(text, /function\s+Assert-NoReparsePointInPath/i);
  assert.match(text, /FileAttributes\]::ReparsePoint/i);
  const check = text.indexOf('Assert-NoReparsePointInPath -Path $destination -Boundary $repoRoot');
  const mutation = text.indexOf('New-Item -ItemType Directory -Force -Path $destination');
  assert.ok(check >= 0 && mutation > check, 'reparse-point preflight must precede staging mutation');
});


test('production endpoint discovery requires complete staged APO signer provenance', () => {
  const text = read('crates/voxveil-windows-audio/src/discovery_windows.rs');
  for (const field of [
    'apo_signer',
    'apo_thumbprint',
    'apo_catalog_signer',
    'apo_catalog_thumbprint',
    'extension_catalog_signer',
    'extension_catalog_thumbprint',
  ]) {
    assert.match(text, new RegExp(field, 'i'));
  }
  assert.match(text, /fn\s+is_certificate_thumbprint/i);
  assert.match(text, /value\.len\(\)\s*==\s*40/i);
  assert.match(text, /!verification\.apo_signer\.trim\(\)\.is_empty\(\)/i);
});
