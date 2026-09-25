import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const relay = () => [
  'crates/voxveil-windows-audio/src/relay.rs',
  'crates/voxveil-windows-audio/src/relay_probe.rs',
  'crates/voxveil-windows-audio/src/relay_support.rs',
  'crates/voxveil-windows-audio/src/relay_control.rs',
].map((path) => readFileSync(path, 'utf8')).join('\n');

test('non-ready Windows audio probes shut down active processing', () => {
  const text = relay();
  assert.match(text, /fn\s+should_fail_closed_after_probe\s*\(/);
  assert.match(text, /readiness\s*!=\s*RelayReadiness::Ready/);
  assert.match(text, /fn\s+disable_processing_best_effort\s*\(&mut self\)/);
  assert.match(text, /self\.relay\.take\(\)/);
  assert.match(text, /run_control\(&control,\s*&\["enabled",\s*"0"\]\)/);
  assert.match(
    text,
    /if\s+should_fail_closed_after_probe\(decision\.readiness\)\s*\{\s*self\.disable_processing_best_effort\(\)/s,
  );
});

test('endpoint enumeration faults also fail closed before returning', () => {
  const text = relay();
  assert.match(
    text,
    /Err\(error\)\s*=>\s*\{\s*self\.disable_processing_best_effort\(\);\s*return\s+fault_probe/s,
  );
});

test('loaded APO readiness is scoped to its persisted default endpoint', () => {
  const text = relay();
  assert.match(text, /apo_covers_default_endpoint/);
  assert.match(text, /load_installed_apo_endpoint/);
  assert.match(text, /query_apo_coverage/);
  assert.match(text, /install-state\.json|system_audio_directory/);
});

test('a loaded APO on another endpoint is disabled before virtual relay processing', () => {
  const text = relay();
  assert.match(text, /loaded_instances\s*>\s*0/);
  assert.match(text, /set_apo_enabled\(false\)/);
  assert.match(text, /apo_covers_default/);
});
