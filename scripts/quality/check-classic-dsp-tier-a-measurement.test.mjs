import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { measureControlledFixture } from '../evaluation/measure-tier-a-controlled-fixture.mjs';

const raw=(values)=>{const b=Buffer.alloc(values.length*4);values.forEach((v,i)=>b.writeFloatLE(v,i*4));return b;};
const sha=(b)=>createHash('sha256').update(b).digest('hex');

test('Tier-A measurement recovers known projection gains from aligned raw references', async()=>{
  const root=await mkdtemp(path.join(os.tmpdir(),'voxveil-tier-a-'));
  try {
    for (const dir of ['fixtures','renders','manifests','measurements']) await mkdir(path.join(root,dir),{recursive:true});
    const vocal=raw([1,-1,1,-1,1,-1,1,-1]);
    const acc=raw([1,1,-1,-1,-1,-1,1,1]);
    const music=raw([1.5,0.5,-0.5,-1.5,-0.5,-1.5,1.5,0.5]);
    const balanced=raw([1.05,0.55,-0.55,-1.05,-0.55,-1.05,1.05,0.55]);
    const strong=raw([0.85,0.65,-0.65,-0.85,-0.65,-0.85,0.85,0.65]);
    await writeFile(path.join(root,'fixtures','f-vocal-reference.f32'),vocal);
    await writeFile(path.join(root,'fixtures','f-accompaniment-reference.f32'),acc);
    await writeFile(path.join(root,'renders','f-music.f32'),music);
    await writeFile(path.join(root,'renders','f-balanced.f32'),balanced);
    await writeFile(path.join(root,'renders','f-strong.f32'),strong);
    await writeFile(path.join(root,'manifests','f.json'),JSON.stringify({
      fixtureId:'f',tier:'controlled',targetSampleRate:44100,
      referenceFiles:{
        vocal:{rawFile:'fixtures/f-vocal-reference.f32',sha256:sha(vocal),bytes:vocal.length},
        accompaniment:{rawFile:'fixtures/f-accompaniment-reference.f32',sha256:sha(acc),bytes:acc.length}
      }
    }));
    await writeFile(path.join(root,'measurements','f-44100-render-evidence.json'),JSON.stringify({
      fixtureId:'f',tier:'controlled',sampleRate:44100,vocal:0,objectiveStatus:'rendered',frameCountParity:true,
      renders:{
        musicPreservation:{rawFile:'renders/f-music.f32',sha256:sha(music)},
        balanced:{rawFile:'renders/f-balanced.f32',sha256:sha(balanced)},
        strong:{rawFile:'renders/f-strong.f32',sha256:sha(strong)}
      }
    }));
    const result=await measureControlledFixture(root,'f');
    const m=result.output.renders.musicPreservation.metrics;
    const b=result.output.renders.balanced.metrics;
    assert.ok(Math.abs(m.vocalProjectionGain-0.5)<1e-6);
    assert.ok(Math.abs(m.accompanimentProjectionGain-1)<1e-6);
    assert.ok(Math.abs(m.vocalAttenuationDb-6.020599913)<1e-6);
    assert.ok(Math.abs(b.vocalProjectionGain-0.25)<1e-6);
    assert.ok(Math.abs(b.accompanimentProjectionGain-0.8)<1e-6);
    assert.ok(Math.abs(b.vocalAttenuationDb-12.041199827)<2e-6);
    assert.ok(result.output.renders.strong.metrics.vocalProjectionGain < b.vocalProjectionGain);
  } finally {
    await rm(root,{recursive:true,force:true});
  }
});
