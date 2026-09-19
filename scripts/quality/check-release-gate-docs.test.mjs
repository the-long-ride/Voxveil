import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const driver = readFileSync('docs/release/windows-driver-signing.md', 'utf8');
const dsp = readFileSync('docs/testing/classic-dsp-evaluation.md', 'utf8');

for (const [name, content] of [['driver signing guide', driver], ['Classic DSP guide', dsp]]) {
  test(`${name} documents the full unified release gate and archive handoff`, () => {
    assert.match(content, /final Windows package/i);
    assert.match(content, /Classic DSP/i);
    assert.match(content, /Windows-driver|signed-driver/i);
    assert.match(content, /signed APO|APO\/CAPX/i);
    assert.match(content, /not applicable/i);
    assert.match(content, /evaluation:export-release-evidence/i);
    assert.match(content, /windows-release-evidence\.md/i);
  });
}

test('release guides no longer describe the unified gate as only two evidence auditors/domains', () => {
  assert.doesNotMatch(dsp, /imports both evidence auditors directly/i);
  assert.doesNotMatch(dsp, /unless both Classic DSP and Windows-driver evidence pass/i);
  assert.doesNotMatch(driver, /if the Classic DSP evidence matrix\/semantic review is incomplete, or if the signed-driver lifecycle\/qualification evidence is incomplete\. A successful result/i);
});
