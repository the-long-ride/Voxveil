import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Classic DSP profile is a backward-compatible Windows audio preference', () => {
  const config = read('tauri/config/windows_audio.rs');

  assert.match(config, /use voxveil_types::ClassicSuppressionProfile;/);
  assert.match(
    config,
    /#\[serde\(default\)\]\s*pub classic_suppression_profile:\s*ClassicSuppressionProfile/,
  );
  assert.match(config, /classicSuppressionProfile/);
  assert.match(config, /legacy_preferences_default_to_music_preservation/);
});

test('profile updates persist and roll back the live backend when saving fails', () => {
  const commands = read('tauri/app/commands.rs');
  const start = commands.indexOf('pub fn set_classic_suppression_profile(');
  const end = commands.indexOf('\n#[tauri::command]', start + 1);
  const setter = commands.slice(start, end === -1 ? undefined : end);

  assert.ok(start >= 0, 'profile setter command must exist');
  assert.match(setter, /app:\s*AppHandle/);
  assert.match(setter, /previous_profile/);
  assert.match(setter, /windows_audio::load\(&app\)/);
  assert.match(setter, /controller\.set_classic_suppression_profile\(profile\)/);
  assert.match(setter, /preferences\.classic_suppression_profile\s*=\s*profile/);
  assert.match(setter, /windows_audio::save\(&app,\s*&preferences\)/);
  assert.match(setter, /controller\.set_classic_suppression_profile\(previous_profile\)/);

  const backendUpdate = setter.indexOf('controller.set_classic_suppression_profile(profile)');
  const save = setter.indexOf('windows_audio::save(&app, &preferences)');
  const stateUpdate = setter.lastIndexOf('classic_suppression_profile = profile');
  assert.ok(backendUpdate >= 0 && save > backendUpdate && stateUpdate > save);
});

test('startup reapplies the saved profile to backend and AppState', () => {
  const bootstrap = read('tauri/lib.rs');

  assert.match(bootstrap, /prefs\.classic_suppression_profile/);
  assert.match(
    bootstrap,
    /controller[\s\S]{0,260}set_classic_suppression_profile\(prefs\.classic_suppression_profile\)/,
  );
  assert.match(
    bootstrap,
    /classic_suppression_profile\s*=\s*prefs\.classic_suppression_profile/,
  );
});
