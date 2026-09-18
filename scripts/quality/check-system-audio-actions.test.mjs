import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const actions = readFileSync('tauri/app/system_audio_actions.rs', 'utf8');

test('Windows shell actions pin Explorer to the OS Windows directory', () => {
  assert.match(actions, /voxveil_windows_audio::windows_system_directory\(\)/i);
  assert.match(actions, /parent\(\)/i);
  assert.match(actions, /explorer\.exe/i);
  assert.doesNotMatch(actions, /Command::new\("explorer\.exe"\)/i);
});
