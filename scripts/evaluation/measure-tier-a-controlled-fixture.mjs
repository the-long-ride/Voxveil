#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readFile, writeFile, stat } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';

const HASH_RE = /^[a-f0-9]{64}$/i;
const hashBuffer = (buffer) => createHash('sha256').update(buffer).digest('hex');
const gainDb = (value) => 20 * Math.log10(Math.max(Math.abs(value), 1e-12));

function resolveWithin(workspace, relativePath, rootName) {
  if (typeof relativePath !== 'string' || !relativePath || path.isAbsolute(relativePath)) {
    throw new Error(`Invalid relative evidence path: ${relativePath}`);
  }
  const root = path.resolve(workspace, rootName);
  const resolved = path.resolve(workspace, relativePath);
  const prefix = root.endsWith(path.sep) ? root : root + path.sep;
  if (!resolved.startsWith(prefix)) throw new Error(`Evidence path escapes ${rootName}: ${relativePath}`);
  return resolved;
}

async function readJson(file) {
  return JSON.parse(await readFile(file, 'utf8'));
}

async function readVerifiedRaw(workspace, descriptor, rootName, label) {
  if (!descriptor || !HASH_RE.test(descriptor.sha256 ?? '')) throw new Error(`${label} descriptor is missing a SHA-256`);
  const file = resolveWithin(workspace, descriptor.rawFile, rootName);
  const buffer = await readFile(file);
  const actual = hashBuffer(buffer);
  if (actual !== descriptor.sha256.toLowerCase()) throw new Error(`${label} SHA-256 mismatch: expected ${descriptor.sha256}, got ${actual}`);
  if (descriptor.bytes !== undefined && descriptor.bytes !== buffer.length) throw new Error(`${label} byte count mismatch: expected ${descriptor.bytes}, got ${buffer.length}`);
  if (buffer.length === 0 || buffer.length % 8 !== 0) throw new Error(`${label} must be non-empty stereo f32le data`);
  return { file, buffer, sha256: actual };
}

function projectionMetrics(vocal, accompaniment, render) {
  if (vocal.length !== accompaniment.length || vocal.length !== render.length) {
    throw new Error(`Aligned raw byte counts differ: vocal=${vocal.length}, accompaniment=${accompaniment.length}, render=${render.length}`);
  }
  let vv=0, mm=0, vm=0, yv=0, ym=0, yy=0;
  const samples=render.length/4;
  for (let offset=0; offset<render.length; offset+=4) {
    const v=vocal.readFloatLE(offset), m=accompaniment.readFloatLE(offset), y=render.readFloatLE(offset);
    if (![v,m,y].every(Number.isFinite)) throw new Error(`Non-finite controlled sample at byte offset ${offset}`);
    vv+=v*v; mm+=m*m; vm+=v*m; yv+=y*v; ym+=y*m; yy+=y*y;
  }
  if (vv<=0 || mm<=0) throw new Error('Controlled references must both contain non-zero energy');
  const determinant=vv*mm-vm*vm;
  if (determinant<=vv*mm*1e-8) throw new Error('Controlled vocal/accompaniment references are too collinear for stable projection metrics');
  const vocalGain=(yv*mm-ym*vm)/determinant;
  const accompanimentGain=(ym*vv-yv*vm)/determinant;
  const residualEnergy=Math.max(0,yy-vocalGain*yv-accompanimentGain*ym);
  const accErrorEnergy=Math.max(0,yy+vocalGain*vocalGain*vv+mm-2*vocalGain*yv-2*ym+2*vocalGain*vm);
  const accRms=Math.sqrt(mm/samples);
  const accErrorRms=Math.sqrt(accErrorEnergy/samples);
  return {
    floatSampleCount:samples,
    frameCount:samples/2,
    referenceCorrelation:vm/Math.sqrt(vv*mm),
    vocalProjectionGain:vocalGain,
    vocalAttenuationDb:-gainDb(vocalGain),
    accompanimentProjectionGain:accompanimentGain,
    accompanimentGainChangeDb:gainDb(accompanimentGain),
    accompanimentErrorRms:accErrorRms,
    accompanimentErrorRelativeDb:20*Math.log10(Math.max(accErrorRms/accRms,1e-12)),
    unexplainedResidualRms:Math.sqrt(residualEnergy/samples),
    renderRms:Math.sqrt(yy/samples)
  };
}

export async function measureControlledFixture(workspaceRoot, fixtureId, { force=false }={}) {
  const workspace=path.resolve(workspaceRoot);
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(fixtureId)) throw new Error('Invalid fixture id');
  const manifest=await readJson(path.join(workspace,'manifests',`${fixtureId}.json`));
  if (manifest.fixtureId!==fixtureId || manifest.tier!=='controlled') throw new Error('Selected manifest is not the requested controlled fixture');
  const rate=manifest.targetSampleRate;
  if (![44100,48000].includes(rate) || !manifest.referenceFiles?.vocal || !manifest.referenceFiles?.accompaniment) {
    throw new Error('Controlled manifest sample rate/referenceFiles are invalid');
  }
  const evidence=await readJson(path.join(workspace,'measurements',`${fixtureId}-${rate}-render-evidence.json`));
  if (evidence.fixtureId!==fixtureId || evidence.tier!=='controlled' || evidence.sampleRate!==rate ||
      evidence.vocal!==0 || evidence.objectiveStatus!=='rendered' || evidence.frameCountParity!==true) {
    throw new Error('Render evidence is not an aligned controlled Vocal=0 render for this fixture');
  }
  const vocal=await readVerifiedRaw(workspace,manifest.referenceFiles.vocal,'fixtures','vocal reference');
  const accompaniment=await readVerifiedRaw(workspace,manifest.referenceFiles.accompaniment,'fixtures','accompaniment reference');
  const results={};
  for (const [profile,descriptor] of Object.entries({
    musicPreservation:evidence.renders?.musicPreservation,
    balanced:evidence.renders?.balanced
  })) {
    if (!descriptor || !HASH_RE.test(descriptor.sha256??'')) throw new Error(`${profile} render descriptor is missing a SHA-256`);
    const file=resolveWithin(workspace,descriptor.rawFile,'renders');
    const buffer=await readFile(file);
    const sha256=hashBuffer(buffer);
    if (sha256!==descriptor.sha256.toLowerCase()) throw new Error(`${profile} render SHA-256 mismatch`);
    const metrics=projectionMetrics(vocal.buffer,accompaniment.buffer,buffer);
    for (const value of Object.values(metrics)) if (typeof value!=='number' || !Number.isFinite(value)) throw new Error(`${profile} produced a non-finite metric`);
    results[profile]={rawFile:descriptor.rawFile,sha256,metrics};
  }
  const outputPath=path.join(workspace,'measurements',`${fixtureId}-${rate}-controlled-metrics.json`);
  try {
    await stat(outputPath);
    if (!force) throw new Error(`Controlled metrics already exist: ${outputPath}. Use --force to replace them.`);
  } catch (error) {
    if (error?.code!=='ENOENT') throw error;
  }
  const output={
    fixtureId,tier:'controlled',sampleRate:rate,vocal:0,generatedAtUtc:new Date().toISOString(),
    method:'global two-source least-squares projection against aligned gained VocalSet and URMP references',
    references:{
      vocal:{rawFile:manifest.referenceFiles.vocal.rawFile,sha256:vocal.sha256},
      accompaniment:{rawFile:manifest.referenceFiles.accompaniment.rawFile,sha256:accompaniment.sha256}
    },
    renders:results,
    notes:'Objective controlled-fixture evidence only. Metrics intentionally define no release threshold and do not replace subjective listening.'
  };
  await writeFile(outputPath,JSON.stringify(output,null,2)+'\n','utf8');
  return {outputPath,output};
}

function parseArgs(argv) {
  let workspace='.local-evaluation/classic-dsp', fixtureId='', force=false, json=false;
  for (let i=0;i<argv.length;i+=1) {
    const arg=argv[i];
    if (arg==='--workspace') workspace=argv[++i];
    else if (arg==='--fixture') fixtureId=argv[++i];
    else if (arg==='--force') force=true;
    else if (arg==='--json') json=true;
    else if (arg==='--help'||arg==='-h') return {help:true,workspace,fixtureId,force,json};
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return {help:false,workspace,fixtureId,force,json};
}

async function main() {
  const args=parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log('Usage: node scripts/evaluation/measure-tier-a-controlled-fixture.mjs --fixture <id> [--workspace <path>] [--force] [--json]');
    return;
  }
  if (!args.fixtureId) throw new Error('--fixture is required');
  const result=await measureControlledFixture(args.workspace,args.fixtureId,{force:args.force});
  if (args.json) console.log(JSON.stringify(result.output,null,2));
  else {
    console.log(`Controlled metrics: ${result.outputPath}`);
    console.log(`Music preservation vocal attenuation: ${result.output.renders.musicPreservation.metrics.vocalAttenuationDb.toFixed(3)} dB`);
    console.log(`Balanced vocal attenuation: ${result.output.renders.balanced.metrics.vocalAttenuationDb.toFixed(3)} dB`);
  }
}

const invokedPath=process.argv[1]?pathToFileURL(path.resolve(process.argv[1])).href:'';
if (invokedPath===import.meta.url) await main();
