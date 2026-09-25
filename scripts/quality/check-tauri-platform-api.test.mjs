import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Tauri platform module re-exports controller types consumed by app commands', () => {
  const commands = read('tauri/app/commands.rs');
  const platform = read('tauri/platform/mod.rs');

  assert.match(commands, /use crate::platform::\{[^}]*PhysicalOutput[^}]*ProcessingController[^}]*\};/s);
  assert.match(platform, /pub use controller::\{[^}]*PhysicalOutput[^}]*\};/s);
});
