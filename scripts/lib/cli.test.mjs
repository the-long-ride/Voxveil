import assert from 'node:assert/strict';
import test from 'node:test';
import { readNamedArgs } from './cli.mjs';

test('reads required named arguments independent of order', () => {
  assert.deepEqual(readNamedArgs(['--edition', 'standard', '--platform', 'linux'], ['platform', 'edition']), {
    platform: 'linux',
    edition: 'standard',
  });
});

test('rejects missing argument values', () => {
  assert.throws(() => readNamedArgs(['--platform', 'linux'], ['platform', 'edition']), /Missing --edition/);
});
