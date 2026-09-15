import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const relay = () => readFileSync('crates/voxveil-windows-audio/src/relay.rs', 'utf8');

test('APO status command failures are not silently interpreted as zero loaded instances', () => {
  const text = relay();
  assert.match(text, /fn\s+loaded_apo_instances\s*\(\)\s*->\s*Result<u32,\s*String>/);
  assert.match(text, /let\s+Some\(control\)\s*=\s*control_executable\(\)\s+else\s*\{\s*return\s+Ok\(0\)/s);
  assert.match(text, /let\s+status\s*=\s*run_control\(&control,\s*&\["status"\]\)\?/);
  assert.match(text, /parse_loaded_instances_required\(&status\)/);
  assert.doesNotMatch(
    text,
    /run_control\(&control,\s*&\["status"\]\)\.ok\(\)[\s\S]{0,160}unwrap_or\(0\)/,
  );
});

test('malformed APO status is an explicit parser error', () => {
  const text = relay();
  assert.match(text, /fn\s+parse_loaded_instances_required\s*\([^)]*\)\s*->\s*Result<u32,\s*String>/);
  assert.match(text, /loaded=.*status/i);
});
