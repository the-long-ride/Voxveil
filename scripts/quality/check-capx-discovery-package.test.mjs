import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const discovery = [
  'crates/voxveil-windows-audio/src/discovery.rs',
  'crates/voxveil-windows-audio/src/discovery_windows.rs',
].map((path) => readFileSync(path, 'utf8')).join('\n');
const runtimeContract = readFileSync('scripts/quality/system-audio-runtime-interface-binding.test.mjs', 'utf8');

test('production endpoint discovery recognizes signed CAPX AddInterface packages', () => {
  assert.match(discovery, /fn capx_extension_inf_matches\s*\(/);
  assert.match(discovery, /addinterface/i);
  assert.match(discovery, /63E268CE-4CBC-48E0-BEB6-55103316F477/i);
  assert.match(discovery, /voxveil_apo_context/i);
  assert.doesNotMatch(discovery, /fn runtime_extension_inf_matches\s*\(/);
});

test('production package matching uses CAPX identity for both runtime and fallback binding', () => {
  assert.match(discovery, /capx_extension_inf_matches\(\s*&text,\s*&?hardware_ids,/s);
  assert.doesNotMatch(discovery, /runtime_extension_inf_matches\(\s*&text/s);
});

test('runtime-interface quality contract distinguishes production CAPX from legacy test attachment', () => {
  assert.match(runtimeContract, /CAPX/i);
  assert.match(runtimeContract, /AddInterface/i);
  assert.match(runtimeContract, /\$TestSign/);
  assert.doesNotMatch(runtimeContract, /doesNotMatch\(extensionTemplate,\s*\/\^\\s\*AddInterface/);
});
