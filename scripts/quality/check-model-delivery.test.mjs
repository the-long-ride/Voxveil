import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectCatalog, inspectModelFiles } from './check-model-delivery.mjs';

const approved = [{
  id: 'htdemucs-ft-vocals-fp16',
  fileName: 'htdemucs_ft_vocals_fp16weights.onnx',
  downloadUrl: 'https://huggingface.co/example/resolve/0123456789abcdef0123456789abcdef01234567/model.onnx',
  sourceRevision: '0123456789abcdef0123456789abcdef01234567',
  license: 'MIT',
  sha256: '0'.repeat(64),
  approximateSizeMb: 166,
  maxBytes: 209715200,
  commercialUse: true,
  directDownloadOnly: true,
}];

test('accepts pinned HTTPS catalog entries with commercial-use metadata', () => {
  assert.deepEqual(inspectCatalog(approved), []);
});

test('rejects unpinned or non-commercial model metadata', () => {
  const issues = inspectCatalog([{ ...approved[0], downloadUrl: 'http://example.com/resolve/0123456789abcdef0123456789abcdef01234567/model.onnx', commercialUse: false, sha256: 'bad' }]);
  assert.match(issues.join('\n'), /HTTPS/);
  assert.match(issues.join('\n'), /commercial/);
  assert.match(issues.join('\n'), /SHA-256/);
});

test('rejects model binaries from repository or bundle resources', () => {
  assert.match(inspectModelFiles(['models/foo.onnx'], []).join('\n'), /model binary/);
  assert.match(inspectModelFiles([], ['models/foo.onnx']).join('\n'), /bundle resource/);
});


test('rejects a catalog URL that does not pin its declared source revision', () => {
  const issues = inspectCatalog([{ ...approved[0], downloadUrl: 'https://huggingface.co/example/resolve/main/model.onnx' }]);
  assert.match(issues.join('\n'), /pin the declared source revision/);
});
