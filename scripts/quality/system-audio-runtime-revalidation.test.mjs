import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const installer = await readFile(
  new URL('../windows/install-system-audio-component.ps1', import.meta.url),
  'utf8',
);

test('elevated installer revalidates the exact native PnP binding', () => {
  assert.match(installer, /runtimeDeviceId\s*=\s*\[string\]\$descriptor\.pnpInstanceId/);
  assert.match(installer, /runtimeAliasMatch\s*=\s*\$true/);
  assert.doesNotMatch(installer, /\$currentTopology\.Count\s*-ne\s*1/);
  assert.match(
    installer,
    /currentTopology[\s\S]*descriptor\.topologyReference/,
  );
});
