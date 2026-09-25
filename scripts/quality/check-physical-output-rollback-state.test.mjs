import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8').replace(/\r\n?/g, '\n');

test('physical-output persistence failure applies authoritative rollback backend state', () => {
  const actions = read('tauri/app/system_audio_actions.rs');
  const saveFailure = actions.match(/if let Err\(error\) = crate::config::windows_audio::save[\s\S]*?\n    \}\n\n/)?.[0] ?? '';

  assert.match(saveFailure, /controller\.set_physical_output\(previous_endpoint_id\)/);
  assert.match(saveFailure, /rollback_snapshot/);
  assert.match(saveFailure, /state\.lock\(\)\?\.apply_backend\(&rollback_snapshot\)/);
  assert.doesNotMatch(saveFailure, /let\s+_\s*=\s*controller\.set_physical_output/);
});

test('UI updates from the output command and avoids re-enumerating on failure', () => {
  const hook = read('ui/app/useVoxveilState.ts');
  const selection = hook.match(/const selectPhysicalOutput[\s\S]*?\n  \}, \[[^\]]+\]\);/)?.[0] ?? '';

  assert.match(selection, /setState\(await client\.setPhysicalAudioOutput\(endpointId\)\)/);
  assert.match(selection, /catch \(error\)[\s\S]*setSystemAudioInstallError\(errorMessage\(error\)\)[\s\S]*client\.getState\(\)/);
  assert.doesNotMatch(selection, /refreshNativeState|refreshSystemAudioEndpoints|refreshPhysicalOutputs/);
});
