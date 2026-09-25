import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Windows backend preserves physical-output enumeration errors', () => {
  const relay = read('crates/voxveil-windows-audio/src/relay.rs');
  const method = relay.match(/pub fn physical_outputs\([\s\S]*?\n    \}/)?.[0] ?? '';

  assert.match(method, /->\s*Result<Vec<EndpointDescriptor>,\s*String>/);
  assert.match(method, /enumerate_render_blocking\(\)\?/);
  assert.doesNotMatch(method, /unwrap_or_default\(\)/);
});

test('non-Windows backend keeps the same fallible physical-output contract', () => {
  const fallback = read('crates/voxveil-windows-audio/src/lib.rs');
  const method = fallback.match(/pub fn physical_outputs\([\s\S]*?\n    \}/)?.[0] ?? '';

  assert.match(method, /->\s*Result<Vec<EndpointDescriptor>,\s*String>/);
  assert.match(method, /Ok\(Vec::new\(\)\)/);
});

test('controller and Tauri command propagate physical-output enumeration failures', () => {
  const controller = read('tauri/platform/controller.rs');
  const commands = read('tauri/app/commands.rs');

  const controllerMethod = controller.match(/pub fn physical_outputs\([\s\S]*?\n    \}/)?.[0] ?? '';
  const command = commands.match(/pub (?:async )?fn list_audio_outputs\([\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(controllerMethod, /->\s*Result<Vec<PhysicalOutput>,\s*String>/);
  assert.match(controllerMethod, /Windows audio backend lock is poisoned/);
  assert.match(controllerMethod, /physical_outputs\(\)\?/);
  assert.doesNotMatch(controllerMethod, /unwrap_or_default\(\)/);

  assert.match(command, /controller\s*\.physical_outputs\(\)\?/s);
});
