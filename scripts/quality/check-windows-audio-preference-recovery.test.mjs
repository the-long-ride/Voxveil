import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = () => readFileSync('tauri/config/windows_audio.rs', 'utf8');

test('malformed Windows audio preference JSON falls back without swallowing read failures', () => {
  const text = source();
  const load = text.match(/pub fn load\([\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(load, /fs::read\(&path\)[\s\S]{0,180}\?/);
  assert.match(load, /serde_json::from_slice\(&bytes\)[\s\S]{0,120}unwrap_or_default\(\)/);
  assert.doesNotMatch(load, /fs::read\(&path\)[\s\S]{0,120}unwrap_or_default\(\)/);
});
