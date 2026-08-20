import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const installer = await readFile(
  new URL('../windows/install-system-audio-component.ps1', import.meta.url),
  'utf8',
);

test('elevated installer revalidates the exact native PnP binding', () => {
  assert.match(
    installer,
    /runtimeDeviceId\s*=\s*\[string\]\$descriptor\.bindingPnpInstanceId/,
  );
  assert.match(installer, /runtimeAliasMatch\s*=\s*\$runtimeBound/);
  assert.match(
    installer,
    /resolved\.bindingPnpInstanceId[\s\S]*descriptor\.bindingPnpInstanceId/,
  );
  assert.match(
    installer,
    /resolved\.pnpInstanceId[\s\S]*descriptor\.pnpInstanceId/,
  );
  assert.match(installer, /topologyInterfacePath/);
  assert.match(installer, /audioInterfacePath/);
  assert.match(installer, /attach-effects/);
  assert.doesNotMatch(installer, /\$currentTopology\.Count\s*-ne\s*1/);
  assert.match(
    installer,
    /if \(-not \$runtimeBound\)[\s\S]*currentTopology[\s\S]*\$topologyReference/,
  );
});
