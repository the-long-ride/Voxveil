import assert from 'node:assert/strict';
import { test } from 'node:test';
import { flattenKeys, compareLocaleKeys } from './check-i18n.mjs';

test('flattens nested locale keys deterministically', () => {
  assert.deepEqual(flattenKeys({ nav: { home: 'Home' }, app: { name: 'Voxveil' } }), [
    'app.name', 'nav.home',
  ]);
});

test('detects missing and extra translation keys', () => {
  assert.deepEqual(compareLocaleKeys(['a', 'b'], ['b', 'c']), {
    missing: ['a'], extra: ['c'],
  });
});
