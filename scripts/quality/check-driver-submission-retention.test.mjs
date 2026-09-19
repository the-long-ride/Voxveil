import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const workflow = readFileSync('.github/workflows/manual-build.yml', 'utf8');
const signingGuide = readFileSync('docs/release/windows-driver-signing.md', 'utf8');

test('exact unsigned driver signing handoff is retained for at least 30 days', () => {
  const start = workflow.indexOf('- name: Upload unsigned driver submission');
  const nextStep = workflow.indexOf('\n      - name:', start + 1);
  const block = workflow.slice(start, nextStep >= 0 ? nextStep : workflow.length);

  assert.ok(start >= 0, 'unsigned driver submission upload step must exist');
  assert.match(block, /Voxveil-windows-driver-submission-\$\{\{ github\.sha \}\}/);
  const retention = Number(block.match(/retention-days:\s*(\d+)/)?.[1] ?? 0);
  assert.ok(retention >= 30, 'signing handoff artifact must be retained for at least 30 days');
  assert.match(signingGuide, /30 days/i);
});
