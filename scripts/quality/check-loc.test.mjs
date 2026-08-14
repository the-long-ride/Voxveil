import assert from 'node:assert/strict';
import { test } from 'node:test';
import { countPhysicalLines, limitFor, violationFor } from './check-loc.mjs';

test('maps hard LOC limits by extension', () => {
  assert.equal(limitFor('a.ts'), 300);
  assert.equal(limitFor('a.tsx'), 400);
  assert.equal(limitFor('a.rs'), 300);
  assert.equal(limitFor('a.css'), 400);
  assert.equal(limitFor('a.md'), null);
});

test('counts blank physical lines', () => {
  assert.equal(countPhysicalLines('a\n\nb\n'), 3);
});

test('flags only values above the limit', () => {
  assert.equal(violationFor('a.ts', 'x\n'.repeat(300)), null);
  assert.deepEqual(violationFor('a.ts', 'x\n'.repeat(301)), {
    path: 'a.ts', lines: 301, limit: 300,
  });
});
