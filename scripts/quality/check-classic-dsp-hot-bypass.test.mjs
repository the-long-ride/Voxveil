import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = () => readFileSync('crates/voxveil-dsp/src/spectral_center.rs', 'utf8');

test('setting Vocal to full transparency clears retained spectral attenuation', () => {
  const text = source();
  const setter = text.match(/pub fn set_vocal_level\([\s\S]*?\n    \}/)?.[0] ?? '';

  assert.match(setter, /vocal_level\.get\(\)\s*>=\s*1\.0/);
  assert.match(setter, /smoothed_gain\.fill\(1\.0\)/);
  assert.doesNotMatch(setter, /allow\(dead_code\)/i);
});

test('Classic DSP spec defines live full-vocal transparency', () => {
  const spec = readFileSync('docs/specs/audio/classic-dsp.md', 'utf8');
  assert.match(
    spec,
    /live transition to `vocalLevel = 1`[\s\S]*retained smoothed spectral gains to unity/i,
  );
  assert.match(spec, /without rebuilding the processor/i);
});
