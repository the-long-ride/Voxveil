import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Windows vocal control propagates live relay and APO failures before committing state', () => {
  const relay = read('crates/voxveil-windows-audio/src/relay.rs');
  const setter = relay.match(/pub fn set_vocal_level\([\s\S]*?\n    \}/)?.[0] ?? '';

  assert.match(setter, /->\s*Result<\(\),\s*String>/);
  assert.match(setter, /relay\.set_vocal_level\([^)]*\)\?/);
  assert.match(setter, /else if let Some\(control\) = control_executable\(\)/);
  assert.match(setter, /run_control\([^;]+\)\?/s);
  assert.doesNotMatch(setter, /let\s+_\s*=\s*(?:relay\.set_vocal_level|run_control)/);
  assert.ok(
    setter.indexOf('self.vocal_level =') > setter.indexOf('run_control'),
    'backend vocal state must be committed only after the live control path succeeds',
  );
});

test('controller and Tauri command preserve vocal-control errors', () => {
  const controller = read('tauri/platform/controller.rs');
  const commands = read('tauri/app/commands.rs');
  const fallback = read('crates/voxveil-windows-audio/src/lib.rs');

  assert.match(controller, /return backend\.set_vocal_level\(value\);/);
  assert.match(commands, /controller\.set_vocal_level\(value\)\?/);
  assert.match(fallback, /pub fn set_vocal_level\(&self, _value: u8\) -> Result<\(\), String>/);
});
