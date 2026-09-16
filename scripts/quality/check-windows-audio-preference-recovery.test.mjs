import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = () => readFileSync('tauri/config/windows_audio.rs', 'utf8');
const bootstrap = () => readFileSync('tauri/lib.rs', 'utf8');

test('malformed Windows audio preference JSON falls back without swallowing read failures', () => {
  const text = source();
  const load = text.match(/pub fn load\([\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(load, /fs::read\(&path\)[\s\S]{0,180}\?/);
  assert.match(load, /serde_json::from_slice\(&bytes\)[\s\S]{0,120}unwrap_or_default\(\)/);
  assert.doesNotMatch(load, /fs::read\(&path\)[\s\S]{0,120}unwrap_or_default\(\)/);
});

test('startup clears a saved physical output only when the endpoint is actually stale', () => {
  const text = bootstrap();

  assert.match(
    text,
    /fn saved_physical_output_is_stale_error\([\s\S]{0,220}selected physical playback endpoint is no longer available/,
  );
  assert.match(
    text,
    /set_physical_output\(Some\(endpoint_id\)\)[\s\S]{0,220}Err\(error\)[\s\S]{0,160}saved_physical_output_is_stale_error\(&error\)/,
  );
  assert.doesNotMatch(
    text,
    /set_physical_output\(Some\(endpoint_id\)\)[\s\S]{0,120}\.is_err\(\)/,
  );
});
