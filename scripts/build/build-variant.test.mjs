import assert from 'node:assert/strict';
import test from 'node:test';

import { commandsForVariant } from './build-variant.mjs';

test('desktop variants map to Tauri build with platform config', () => {
  assert.deepEqual(commandsForVariant('windows', 'standard'), [
    ['run', 'tauri', '--workspace', '@voxveil/tauri-shell', '--', 'build', '--config', 'tauri.windows.conf.json'],
  ]);
});

test('mobile variants initialize and build their target', () => {
  assert.deepEqual(commandsForVariant('android', 'pro-system'), [
    ['run', 'tauri', '--workspace', '@voxveil/tauri-shell', '--', 'android', 'init'],
    ['run', 'tauri', '--workspace', '@voxveil/tauri-shell', '--', 'android', 'build', '--config', 'tauri.android.conf.json'],
  ]);
  assert.deepEqual(commandsForVariant('ios', 'standard')[1], [
    'run', 'tauri', '--workspace', '@voxveil/tauri-shell', '--', 'ios', 'build', '--config', 'tauri.ios.conf.json',
  ]);
});

test('unknown platforms and editions are rejected', () => {
  assert.throws(() => commandsForVariant('solaris', 'standard'), /platform/i);
  assert.throws(() => commandsForVariant('linux', 'enterprise'), /edition/i);
});
