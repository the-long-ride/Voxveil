import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Classic DSP real-audio fixtures stay in one ignored local workspace', () => {
  const gitignore = read('.gitignore');
  const corpus = read('docs/testing/classic-dsp-fixture-corpus.md');
  const evaluation = read('docs/testing/classic-dsp-evaluation.md');

  assert.match(gitignore, /^\.local-evaluation\/$/m);
  assert.match(corpus, /\.local-evaluation\/classic-dsp/i);
  assert.match(evaluation, /\.local-evaluation\/classic-dsp/i);
  assert.match(corpus, /do not commit/i);
});
