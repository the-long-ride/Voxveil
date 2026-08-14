import assert from 'node:assert/strict';
import test from 'node:test';
import { approvalErrors } from './dependency-policy.mjs';

test('accepts commercially approved permissive licenses', () => {
  assert.deepEqual(approvalErrors('react', { license: 'MIT', commercialUse: true }), []);
});

test('rejects non-commercial or ambiguous approvals', () => {
  assert.match(approvalErrors('model', { license: 'CC-BY-NC-4.0', commercialUse: false }).join('\n'), /commercial use/);
  assert.match(approvalErrors('model', { license: 'Unknown', commercialUse: true }).join('\n'), /license metadata/);
});
