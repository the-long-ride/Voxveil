import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const schemaPath = 'docs/testing/classic-dsp-fixture-manifest.schema.json';
const read = (path) => readFileSync(path, 'utf8');

test('Classic DSP fixture manifests have a committed machine-readable contract', () => {
  assert.ok(existsSync(schemaPath), `${schemaPath} must exist`);

  const schema = JSON.parse(read(schemaPath));
  const required = new Set(schema.required ?? []);
  for (const field of [
    'fixtureId',
    'tier',
    'status',
    'targetSampleRate',
    'durationSeconds',
    'sources',
    'mixRecipe',
    'fixtureSha256',
    'notes',
  ]) {
    assert.ok(required.has(field), `manifest schema must require ${field}`);
  }

  assert.deepEqual(schema.properties?.tier?.enum, ['controlled', 'natural-mix']);
  assert.deepEqual(schema.properties?.targetSampleRate?.enum, [44100, 48000]);
  assert.equal(schema.properties?.sources?.minItems, 1);
  assert.match(
    schema.properties?.fixtureSha256?.pattern ?? '',
    /64/,
    'fixture hash must require a SHA-256-shaped value',
  );
});

test('controlled fixture recipes require enough metadata to reproduce the mix', () => {
  const schema = JSON.parse(read(schemaPath));
  const recipeVariants = schema.properties?.mixRecipe?.oneOf ?? [];
  const controlledRecipe = recipeVariants.find((variant) => variant?.type === 'object');
  assert.ok(controlledRecipe, 'mixRecipe must define an object variant for controlled fixtures');

  const required = new Set(controlledRecipe.required ?? []);
  for (const field of [
    'vocalStartSeconds',
    'accompanimentStartSeconds',
    'vocalGainDb',
    'accompanimentGainDb',
    'vocalPan',
    'normalization',
    'preparationCommand',
    'preparationToolVersion',
  ]) {
    assert.ok(required.has(field), `controlled mixRecipe must require ${field}`);
  }
});

test('fixture-corpus policy points local manifests at the schema', () => {
  const corpus = read('docs/testing/classic-dsp-fixture-corpus.md');
  assert.match(corpus, /classic-dsp-fixture-manifest\.schema\.json/);
  assert.match(corpus, /conform|schema/i);
  assert.match(corpus, /natural-mix[\s\S]{0,120}mixRecipe/i);
});
