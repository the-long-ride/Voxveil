import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Windows audio test helpers are not compiled into production builds', () => {
  const device = read('crates/voxveil-windows-audio/src/device.rs');
  const discovery = read('crates/voxveil-windows-audio/src/discovery.rs');
  const relay = read('crates/voxveil-windows-audio/src/relay.rs');
  const relayTests = read('crates/voxveil-windows-audio/src/relay_tests.rs');
  const relayEngine = read('crates/voxveil-windows-audio/src/relay_engine.rs');
  const wasapiRelay = read('crates/voxveil-windows-audio/src/wasapi_relay.rs');
  const lib = read('crates/voxveil-windows-audio/src/lib.rs');

  assert.match(device, /#\[cfg\(test\)\]\s*pub\(crate\) fn component_probe/i);
  assert.match(discovery, /#\[cfg\(test\)\]\s*pub\(crate\) fn extension_inf_matches/i);
  assert.match(
    relayEngine,
    /impl RelayRuntimeState\s*\{\s*#\[cfg\(test\)\]\s*pub\(crate\) fn is_running/i,
  );
  assert.match(relayEngine, /#\[cfg\(test\)\]\s*pub\(crate\) fn spawn_with_worker/i);
  assert.doesNotMatch(relayEngine, /pub\(crate\) fn start_wasapi\s*\(/i);
  assert.doesNotMatch(wasapiRelay, /pub\(crate\) fn run_relay_worker\s*\(/i);
  assert.doesNotMatch(
    relay,
    /use crate::device::\{[^}]*\b(?:RelayReadiness|WindowsInterceptionKind)\b[^}]*\}/s,
  );
  assert.match(
    relayTests,
    /use crate::device::\{\s*RelayReadiness,\s*WindowsInterceptionKind\s*\};/i,
  );
  assert.match(
    lib,
    /#\[cfg\(not\(windows\)\)\]\s*use voxveil_types::\{[^}]*ClassicSuppressionProfile[^}]*\};/i,
  );

  assert.doesNotMatch(device, /#\[allow\(dead_code\)\]/i);
  assert.doesNotMatch(discovery, /#\[allow\(dead_code\)\]/i);
  assert.doesNotMatch(relayEngine, /#\[allow\(dead_code\)\]/i);
  assert.doesNotMatch(wasapiRelay, /#\[allow\(dead_code\)\]/i);
});
