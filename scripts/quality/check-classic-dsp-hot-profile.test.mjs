import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = () => readFileSync('crates/voxveil-dsp/src/spectral_center.rs', 'utf8');

test('switching to Music preservation clears stronger retained attenuation', () => {
  const text = source();
  const setter = text.match(/pub fn set_profile\([\s\S]*?\n    \}/)?.[0] ?? '';

  assert.match(setter, /self\.profile\s*!=\s*ClassicSuppressionProfile::MusicPreservation/);
  assert.match(setter, /profile\s*==\s*ClassicSuppressionProfile::MusicPreservation/);
  assert.match(setter, /smoothed_gain\.fill\(1\.0\)/);
  assert.doesNotMatch(setter, /input_[lr]\.fill|ola_[lr]\.fill|previous_mid_magnitude\.fill/);
});

test('Classic DSP spec forbids inheriting stronger Balanced attenuation in Music preservation', () => {
  const spec = readFileSync('docs/specs/audio/classic-dsp.md', 'utf8');
  assert.match(spec, /Balanced\s*(?:→|->|to)\s*Music preservation/i);
  assert.match(spec, /retained smoothed spectral gains[\s\S]{0,220}unity/i);
  assert.match(spec, /without restarting the stream|without rebuilding the processor/i);
});
