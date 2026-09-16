import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const relay = () => readFileSync('crates/voxveil-windows-audio/src/relay.rs', 'utf8');

test('missing APO control fails closed when an APO install state exists', () => {
  const text = relay();
  const loaded = text.match(/fn loaded_apo_instances\([\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(loaded, /apo_install_state_exists\(\)\?/);
  assert.match(loaded, /control component is unavailable|load state cannot be verified/i);
  assert.ok(
    loaded.indexOf('apo_install_state_exists()?') < loaded.lastIndexOf('Ok(0)'),
    'installed-APO detection must run before treating a missing control helper as zero loaded instances',
  );
});

test('APO install-state presence check distinguishes only NotFound from existing or unreadable state', () => {
  const text = relay();
  const helper = text.match(/fn apo_install_state_exists\([\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(helper, /install-state\.json/);
  assert.match(helper, /Ok\([^)]*\)\s*=>\s*Ok\(true\)/);
  assert.match(helper, /ErrorKind::NotFound/);
  assert.match(helper, /Err\(error\)/);
  assert.doesNotMatch(helper, /is_file\(\)/);
});
