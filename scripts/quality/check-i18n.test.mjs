import assert from 'node:assert/strict';
import { test } from 'node:test';
import { combineLocale, flattenKeys, compareLocaleKeys, validateLocales } from './check-i18n.mjs';

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

test('system audio locale namespace participates in key parity', () => {
  assert.deepEqual(
    flattenKeys(combineLocale({ app: { name: 'Voxveil' } }, { title: 'Windows System Audio', refresh: 'Refresh' })),
    ['app.name', 'systemAudio.refresh', 'systemAudio.title'],
  );
});

test('all repository locales have common and system audio key parity', () => {
  assert.deepEqual(validateLocales('.'), []);
});
