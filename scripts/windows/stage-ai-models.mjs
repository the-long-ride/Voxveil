import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { copyFile, mkdir, readFile, rename, rm, stat, writeFile, cp } from 'node:fs/promises';
import path from 'node:path';
import { Readable, Transform } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { fileURLToPath } from 'node:url';
import { inspectCatalog } from '../quality/check-model-delivery.mjs';

async function verified(file, descriptor) {
  try {
    const info = await stat(file);
    if (!info.isFile() || info.size === 0 || info.size > descriptor.maxBytes) return false;
    const hash = createHash('sha256');
    for await (const bytes of createReadStream(file)) hash.update(bytes);
    return hash.digest('hex') === descriptor.sha256;
  } catch (error) {
    if (error.code === 'ENOENT') return false;
    throw error;
  }
}

async function download(descriptor, destination, fetcher) {
  let url = descriptor.downloadUrl;
  let response;
  for (let redirect = 0; redirect <= 5; redirect += 1) {
    if (!url.startsWith('https://')) throw new Error('Model download requires HTTPS');
    response = await fetcher(url, { redirect: 'manual', signal: AbortSignal.timeout(1_800_000) });
    if (![301, 302, 303, 307, 308].includes(response.status)) break;
    const location = response.headers.get('location');
    url = location ? new URL(location, url).href : '';
    await response.body?.cancel();
    response = undefined;
  }
  if (!response?.ok || !response.body) throw new Error('Model download failed or exceeded redirect limit');
  const temporary = `${destination}.download`;
  let bytes = 0;
  const hash = createHash('sha256');
  try {
    await pipeline(Readable.fromWeb(response.body), new Transform({
      transform(chunk, _, done) {
        bytes += chunk.length;
        if (bytes > descriptor.maxBytes) return done(new Error('Model exceeds catalog size limit'));
        hash.update(chunk);
        done(null, chunk);
      },
    }), createWriteStream(temporary));
    if (!bytes || hash.digest('hex') !== descriptor.sha256) throw new Error('Model SHA-256 mismatch');
    await rename(temporary, destination);
  } finally {
    await rm(temporary, { force: true });
  }
}

export async function stageModels({ repo, destination, fetcher = fetch }) {
  const catalog = JSON.parse(await readFile(path.join(repo, 'tauri/models/catalog.json'), 'utf8'));
  const issues = inspectCatalog(catalog.models);
  if (issues.length) throw new Error(issues.join('\n'));
  const models = catalog.models.filter((model) => model.bundledByDefault === true);
  if (!models.length) throw new Error('No bundled AI models declared in catalog');
  const target = path.join(repo, 'target');
  const cache = path.join(target, 'ai-model-cache');
  const resources = path.join(target, 'bundled-ai-models');
  const staging = `${resources}.partial`;
  await mkdir(cache, { recursive: true });
  await rm(staging, { recursive: true, force: true });
  try {
    for (const model of models) {
      const directory = path.join(staging, model.id);
      await mkdir(directory, { recursive: true });
      for (const file of model.files) {
        const cached = path.join(cache, `${file.sha256}.onnx`);
        if (!await verified(cached, file)) {
          await rm(cached, { force: true });
          await download(file, cached, fetcher);
        }
        const staged = path.join(directory, file.fileName);
        await copyFile(cached, staged);
        if (!await verified(staged, file)) throw new Error('Staged model SHA-256 mismatch');
      }
    }
    await rm(resources, { recursive: true, force: true });
    await rename(staging, resources);
    await writeFile(path.join(target, 'ai-model-resources.json'), JSON.stringify({
      bundle: { resources: { '../target/bundled-ai-models/': 'models/' } },
    }));
    if (destination) {
      await rm(path.join(destination, 'models'), { recursive: true, force: true });
      await cp(resources, path.join(destination, 'models'), { recursive: true });
    }
  } finally {
    await rm(staging, { recursive: true, force: true });
  }
  return resources;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const value = (flag) => args.includes(flag) ? args[args.indexOf(flag) + 1] : undefined;
  await stageModels({ repo: value('--repo') ?? fileURLToPath(new URL('../../', import.meta.url)), destination: value('--destination') });
  console.log('Both bundled AI models staged and SHA-256 verified.');
}
