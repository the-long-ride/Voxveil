import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const modules = ['windows', 'linux', 'macos', 'android', 'ios'];

test('platform modules reuse capability presets instead of duplicating struct literals', async () => {
  for (const platform of modules) {
    const source = await readFile(`tauri/platform/${platform}/mod.rs`, 'utf8');
    assert.doesNotMatch(source, /PlatformCapabilities\s*\{\s*all_output/,
      `${platform} should reuse a capability preset from tauri/platform/mod.rs`);
  }
});

test('communication bypass uses a shared enum instead of magic strings', async () => {
  const dto = await readFile('tauri/app/dto.rs', 'utf8');
  const commands = await readFile('tauri/app/commands.rs', 'utf8');
  assert.match(dto, /AudioBypassReason/);
  assert.doesNotMatch(dto, /bypass_reason:\s*Option<String>/);
  assert.doesNotMatch(commands, /Some\("communication"\)/);
});

test('AppState mutex accessor is named lock, not read, because it yields mutable state', async () => {
  const state = await readFile('tauri/app/state.rs', 'utf8');
  const commands = await readFile('tauri/app/commands.rs', 'utf8');
  assert.match(state, /pub fn lock\(/);
  assert.doesNotMatch(state, /pub fn read\(/);
  assert.doesNotMatch(commands, /state\.read\(\)/);
});

test('foundation integration modules are intentionally public crate surfaces', async () => {
  const lib = await readFile('tauri/lib.rs', 'utf8');
  for (const module of ['audio', 'platform', 'realtime', 'routing', 'security', 'separation']) {
    assert.match(lib, new RegExp(`pub mod ${module};`), `${module} should be reachable for integration use and tests`);
  }
});

test('native application state starts processing disabled', async () => {
  const dto = await readFile('tauri/app/dto.rs', 'utf8');
  assert.match(dto, /master_enabled:\s*false/);
  assert.match(dto, /vocal_level:\s*100/);
  assert.match(dto, /quality:\s*50/);
  assert.match(dto, /output_mode:\s*OutputMode::Physical/);
});

test('foundation queue is clearly scoped as single-thread local storage', async () => {
  const buffer = await readFile('crates/voxveil-audio-core/src/buffer.rs', 'utf8');
  const lib = await readFile('crates/voxveil-audio-core/src/lib.rs', 'utf8');
  assert.match(buffer, /pub struct LocalFixedQueue/);
  assert.match(buffer, /single-thread/i);
  assert.match(lib, /LocalFixedQueue/);
  assert.doesNotMatch(lib, /\bFixedQueue\b/);
});

test('Rust DTO exposes domain load state rather than UI translation keys', async () => {
  const dto = await readFile('tauri/app/dto.rs', 'utf8');
  assert.match(dto, /ProcessingLoad/);
  assert.match(dto, /pub load:\s*ProcessingLoad/);
  assert.doesNotMatch(dto, /load_label_key|status\.mediumLoad/);
});

test('backend rejects virtual routes when no virtual output is available', async () => {
  const commands = await readFile('tauri/app/commands.rs', 'utf8');
  assert.match(commands, /virtual_output_available/);
  assert.match(commands, /OutputMode::Virtual\s*\|\s*OutputMode::Both/);
});


test('AI model acquisition is consent-gated, app-local and integrity checked', async () => {
  const commands = await readFile('tauri/models/commands.rs', 'utf8');
  const storage = await readFile('tauri/models/storage.rs', 'utf8');
  const download = await readFile('tauri/models/download.rs', 'utf8');
  const appCommands = await readFile('tauri/app/commands.rs', 'utf8');
  assert.match(commands, /if !accepted_terms/);
  assert.match(storage, /app_local_data_dir\(\)/);
  assert.match(download, /Sha256/);
  assert.match(download, /\.download/);
  assert.match(appCommands, /ai_runtime_ready/);
  const modelModule = await readFile('tauri/models/mod.rs', 'utf8');
  assert.match(modelModule, /AI_RUNTIME_AVAILABLE:\s*bool\s*=\s*false/);
});

test('master processing cannot claim enabled without a ready audio backend', async () => {
  const types = await readFile('crates/voxveil-types/src/processing.rs', 'utf8');
  const dto = await readFile('tauri/app/dto.rs', 'utf8');
  const commands = await readFile('tauri/app/commands.rs', 'utf8');
  const windows = await readFile('tauri/platform/windows/mod.rs', 'utf8');
  assert.match(types, /enum ProcessingBackendStatus/);
  assert.match(dto, /pub backend_status:\s*ProcessingBackendStatus/);
  assert.match(commands, /ProcessingBackendStatus::Ready/);
  assert.match(commands, /processing backend is unavailable/i);
  assert.match(windows, /ProcessingBackendStatus::ComponentRequired/);
});
