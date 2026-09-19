import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const recorder = readFileSync('scripts/evaluation/record-classic-dsp-coverage-review.ps1', 'utf8');

test('Classic DSP coverage recorder references only accepted fixture evidence and never infers semantic labels', () => {
  assert.match(recorder, /subjectiveReview\.status/i);
  assert.match(recorder, /subjectiveReview\.decision/i);
  assert.match(recorder, /decision\s+-ne\s+'accepted'/i);
  assert.match(recorder, /manifestSha256/i);
  assert.match(recorder, /renderEvidenceSha256/i);
  assert.match(recorder, /Coverage categories are explicit human review labels/i);
  assert.match(recorder, /does not infer semantic content/i);
});

test('Classic DSP coverage recorder requires every semantic category and explicit harmony licensing disposition', () => {
  for (const name of [
    'MaleLeadVocalFixture',
    'FemaleLeadVocalFixture',
    'SparseAccompanimentFixture',
    'DenseAccompanimentFixture',
    'CenteredInstrumentFixture',
    'WideStereoAmbienceFixture',
    'MonoNearMonoFixture',
  ]) {
    assert.match(recorder, new RegExp(name, 'i'));
  }
  assert.match(recorder, /HarmonyDoubleTrackedFixture/i);
  assert.match(recorder, /HarmonyCoverageNotApplicableReason/i);
  assert.match(recorder, /either an accepted fixture or an explicit licensing-based not-applicable reason/i);
});
