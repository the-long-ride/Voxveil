import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('Classic DSP profile is a backward-compatible Windows audio preference', () => {
  const config = read('tauri/config/windows_audio.rs');

  assert.match(config, /use voxveil_types::\{[^}]*ClassicSuppressionProfile[^}]*\};/);
  assert.match(
    config,
    /#\[serde\(default\)\]\s*classic_suppression_profile:\s*ClassicSuppressionProfile/,
  );
  assert.match(config, /classicSuppressionProfile/);
  assert.match(config, /legacy_preferences_default_to_music_preservation/);
});

test('Windows profile updates persist and roll back the live backend when saving fails', () => {
  const commands = read('tauri/app/commands.rs');
  const start = commands.search(/pub (?:async )?fn set_classic_suppression_profile\(/);
  const end = commands.indexOf('\n#[tauri::command]', start + 1);
  const setter = commands.slice(start, end === -1 ? undefined : end);

  assert.ok(start >= 0, 'profile setter command must exist');
  assert.match(setter, /app:\s*AppHandle/);
  assert.match(setter, /#\[cfg\(target_os\s*=\s*"windows"\)\]/);
  assert.match(setter, /previous_profile/);
  assert.match(setter, /windows_audio::load\(&app\)/);
  assert.match(setter, /controller\.set_classic_suppression_profile\(profile\)/);
  assert.match(setter, /preferences\.classic_suppression_profile\s*=\s*profile/);
  assert.match(setter, /windows_audio::save\(&app,\s*&preferences\)/);
  assert.match(setter, /controller\.set_classic_suppression_profile\(previous_profile\)/);

  const backendUpdate = setter.indexOf('controller.set_classic_suppression_profile(profile)');
  const save = setter.indexOf('windows_audio::save(&app, &preferences)');
  const stateUpdate = setter.indexOf('classic_suppression_profile = profile', save);
  assert.ok(backendUpdate >= 0 && save > backendUpdate && stateUpdate > save);
});

test('non-Windows profile updates do not depend on Windows audio preferences', () => {
  const commands = read('tauri/app/commands.rs');
  const start = commands.search(/pub (?:async )?fn set_classic_suppression_profile\(/);
  const end = commands.indexOf('\n#[tauri::command]', start + 1);
  const setter = commands.slice(start, end === -1 ? undefined : end);

  assert.match(setter, /#\[cfg\(not\(target_os\s*=\s*"windows"\)\)\]/);
  const nonWindowsStart = setter.indexOf('#[cfg(not(target_os = "windows"))]');
  const nonWindows = nonWindowsStart >= 0 ? setter.slice(nonWindowsStart) : '';
  assert.match(nonWindows, /controller\.set_classic_suppression_profile\(profile\)\?/);
  assert.match(nonWindows, /classic_suppression_profile\s*=\s*profile/);
  assert.doesNotMatch(nonWindows, /windows_audio::(?:load|save)/);
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
