import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Windows audio test helpers are not compiled into production builds', () => {
  const device = read('crates/voxveil-windows-audio/src/device.rs');
  const discovery = read('crates/voxveil-windows-audio/src/discovery.rs');
  const relayEngine = read('crates/voxveil-windows-audio/src/relay_engine.rs');

  assert.match(device, /#\[cfg\(test\)\]\s*pub\(crate\) fn component_probe/i);
  assert.match(discovery, /#\[cfg\(test\)\]\s*pub\(crate\) fn extension_inf_matches/i);
  assert.match(
    relayEngine,
    /impl RelayRuntimeState\s*\{\s*#\[cfg\(test\)\]\s*pub\(crate\) fn is_running/i,
  );

  assert.doesNotMatch(device, /#\[allow\(dead_code\)\]/i);
  assert.doesNotMatch(discovery, /#\[allow\(dead_code\)\]/i);
  assert.doesNotMatch(relayEngine, /#\[allow\(dead_code\)\]/i);
});
