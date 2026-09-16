import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const dto = () => readFileSync('tauri/app/dto.rs', 'utf8');

test('backend snapshots clear a stale physical-output label when no output is resolved', () => {
  const text = dto();
  const applyBackend = text.match(/pub fn apply_backend\([\s\S]*?\n    \}/)?.[0] ?? '';

  assert.match(
    applyBackend,
    /self\.physical_output\s*=\s*snapshot\s*\.physical_output\s*\.clone\(\)\s*\.unwrap_or_else\(\|\|\s*"System Default"\.into\(\)\)/,
  );
  assert.doesNotMatch(applyBackend, /if let Some\(output\) = &snapshot\.physical_output/);
});
