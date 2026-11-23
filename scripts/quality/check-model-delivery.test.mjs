import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { inspectCatalog, inspectModelFiles } from './check-model-delivery.mjs';

const approved = [{
  id: 'spleeter-2stems-fp16',
  sourceRevision: '0123456789abcdef0123456789abcdef01234567',
  license: 'MIT',
  attribution: 'Deezer / sherpa-onnx',
  licenseUrls: ['https://github.com/deezer/spleeter/blob/master/LICENSE'],
  approximateSizeMb: 40,
  files: [{
    fileName: 'vocals.fp16.onnx',
    downloadUrl: 'https://huggingface.co/example/resolve/0123456789abcdef0123456789abcdef01234567/model.onnx',
    sha256: '0'.repeat(64),
    maxBytes: 20971520,
  }],
  commercialUse: true,
  directDownloadOnly: true,
}];

test('accepts pinned HTTPS catalog entries with commercial-use metadata', () => {
  assert.deepEqual(inspectCatalog(approved), []);
});

test('rejects unpinned or non-commercial model metadata', () => {
  const issues = inspectCatalog([{ ...approved[0], commercialUse: false, files: [{ ...approved[0].files[0], downloadUrl: 'http://example.com/model.onnx', sha256: 'bad' }] }]);
  assert.match(issues.join('\n'), /HTTPS/);
  assert.match(issues.join('\n'), /commercial/);
  assert.match(issues.join('\n'), /SHA-256/);
});

test('rejects model binaries from repository or bundle resources', () => {
  assert.match(inspectModelFiles(['models/foo.onnx'], []).join('\n'), /model binary/);
  assert.match(inspectModelFiles([], ['models/foo.onnx']).join('\n'), /bundle resource/);
});


test('rejects a catalog URL that does not pin its declared source revision', () => {
  const issues = inspectCatalog([{ ...approved[0], files: [{ ...approved[0].files[0], downloadUrl: 'https://huggingface.co/example/resolve/main/model.onnx' }] }]);
  assert.match(issues.join('\n'), /pin the declared source revision/);
});

test('validates all files in multi-file models and rejects missing or unsafe file lists', () => {
  const file = approved[0].files[0];
  const files = [file, { ...file, fileName: 'accompaniment.fp16.onnx' }];
  assert.deepEqual(inspectCatalog([{ ...approved[0], files }]), []);
  assert.match(inspectCatalog([{ ...approved[0], files: [...files, file] }]).join('\n'), /unique safe filename/);
  assert.match(inspectCatalog([{ ...approved[0], files: [file, { ...file, fileName: '../escape.onnx', sha256: '' }] }]).join('\n'), /SHA-256/);
  assert.match(inspectCatalog([{ ...approved[0], files: [] }]).join('\n'), /at least one model file/);
  assert.match(inspectCatalog([{ ...approved[0], attribution: '' }]).join('\n'), /attribution/);
});

test('Windows packages stage only the pinned CPU runtime beside the app with upstream notices', () => {
  const staging = readFileSync('scripts/windows/stage-onnx-runtime.ps1', 'utf8');
  assert.match(staging, /microsoft\.ml\.onnxruntime\/1\.23\.2\/microsoft\.ml\.onnxruntime\.1\.23\.2\.nupkg/);
  assert.match(staging, /25fe172f7fcdf34f2b5b02c8b997f6b88abc282956849d4af4335af23d0c4e4a/);
  assert.match(staging, /Get-FileHash[^\n]+-Algorithm SHA256/);
  assert.match(staging, /runtimes\/win-x64\/native\/onnxruntime\.dll/);
  assert.match(staging, /onnxruntime_providers_shared\.dll/);
  assert.match(staging, /ONNX_RUNTIME_LICENSE\.txt/);
  assert.match(staging, /ONNX_RUNTIME_ThirdPartyNotices\.txt/);
  assert.match(staging, /THIRD_PARTY_NOTICES\.md/);
  for (const builder of ['build-windows.ps1', 'build-driverless-package.ps1']) {
    const script = readFileSync(`scripts/windows/${builder}`, 'utf8');
    assert.match(script, /stage-onnx-runtime\.ps1[^\n]+-Destination \$output/);
    assert.ok(script.indexOf('stage-onnx-runtime.ps1') < script.indexOf('$hashLines'));
  }
});
