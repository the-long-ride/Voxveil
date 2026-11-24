import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { stageModels } from './stage-ai-models.mjs';

async function fixture(t) {
  await mkdir('target', { recursive: true });
  const repo = await mkdtemp(path.resolve('target/ai model staging-'));
  t.after(() => rm(repo, { recursive: true, force: true }));
  const catalog = JSON.parse(await readFile('tauri/models/catalog.json', 'utf8'));
  const payloads = new Map();
  for (const model of catalog.models) for (const file of model.files) {
    const bytes = Buffer.from(file.fileName);
    payloads.set(file.downloadUrl, bytes);
    file.sha256 = createHash('sha256').update(bytes).digest('hex');
    file.maxBytes = bytes.length;
  }
  const catalogFile = path.join(repo, 'tauri/models/catalog.json');
  await mkdir(path.dirname(catalogFile), { recursive: true });
  const save = () => writeFile(catalogFile, JSON.stringify(catalog));
  await save();
  const calls = [];
  const fetcher = async (url, options) => {
    calls.push(url);
    assert.equal(options.redirect, 'manual');
    assert.ok(payloads.has(url));
    return new Response(payloads.get(url));
  };
  return { repo, catalog, payloads, calls, fetcher, save };
}

test('stages all three pinned files as Tauri resources and into portable package layout', async (t) => {
  const f = await fixture(t);
  const destination = path.join(f.repo, 'package with spaces');
  await mkdir(destination);
  const resources = await stageModels({ ...f, destination });
  assert.equal(f.calls.length, 3);
  assert.deepEqual(f.catalog.models.map((model) => model.id).sort(), ['mdx-inst-hq3', 'spleeter-2stems-fp16']);
  for (const model of f.catalog.models) for (const file of model.files) {
    const expected = f.payloads.get(file.downloadUrl);
    assert.deepEqual(await readFile(path.join(resources, model.id, file.fileName)), expected);
    assert.deepEqual(await readFile(path.join(destination, 'models', model.id, file.fileName)), expected);
  }
  assert.deepEqual(JSON.parse(await readFile(path.join(f.repo, 'target/ai-model-resources.json'), 'utf8')), {
    bundle: { resources: { '../target/bundled-ai-models/': 'models/' } },
  });
});

test('reuses only hash-verified cache files offline and repairs a corrupt cached model', async (t) => {
  const f = await fixture(t);
  await stageModels(f);
  await stageModels({ ...f, fetcher: () => { throw new Error('Offline'); } });
  const file = f.catalog.models[0].files[0];
  await writeFile(path.join(f.repo, 'target/ai-model-cache', `${file.sha256}.onnx`), Buffer.alloc(file.maxBytes));
  f.calls.length = 0;
  await stageModels(f);
  assert.deepEqual(f.calls, [file.downloadUrl]);
});

for (const [label, response, message] of [
  ['hash mismatch', () => new Response('bad'), /SHA-256/],
  ['empty download', () => new Response(''), /SHA-256/],
  ['oversized download', () => new Response('x'.repeat(100)), /size limit/],
  ['HTTP error', () => new Response(null, { status: 500 }), /download failed/],
  ['HTTPS downgrade', () => new Response(null, { status: 302, headers: { location: 'http://example.com/model.onnx' } }), /requires HTTPS/],
  ['redirect loop', () => new Response(null, { status: 302, headers: { location: 'https://example.com/model.onnx' } }), /redirect limit/],
  ['transport failure', () => { throw new Error('Offline'); }, /Offline/],
]) {
  test(`refuses ${label} and never publishes unverified resources`, async (t) => {
    const f = await fixture(t);
    await assert.rejects(stageModels({ ...f, fetcher: response }), message);
    for (const name of ['bundled-ai-models', 'bundled-ai-models.partial', 'ai-model-resources.json']) {
      await assert.rejects(stat(path.join(f.repo, 'target', name)), { code: 'ENOENT' });
    }
    const file = f.catalog.models[0].files[0];
    await assert.rejects(stat(path.join(f.repo, 'target/ai-model-cache', `${file.sha256}.onnx.download`)), { code: 'ENOENT' });
  });
}

test('empty bundled catalog and unsafe metadata fail before making network requests', async (t) => {
  const f = await fixture(t);
  for (const model of f.catalog.models) model.bundledByDefault = false;
  await f.save();
  await assert.rejects(stageModels(f), /No bundled AI models/);
  f.catalog.models[0].bundledByDefault = true;
  f.catalog.models[0].id = '../outside';
  await f.save();
  await assert.rejects(stageModels(f), /path-safe/);
  assert.equal(f.calls.length, 0);
});
