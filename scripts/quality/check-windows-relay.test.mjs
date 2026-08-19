import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const read = (file) => readFile(new URL(`../../${file}`, import.meta.url), 'utf8');

test('workspace includes isolated Windows audio crate', async () => {
  const cargo = await read('Cargo.toml');
  assert.match(cargo, /crates\/voxveil-windows-audio/);
});

test('Windows audio crate uses exact MIT WASAPI dependency only on Windows', async () => {
  const cargo = await read('crates/voxveil-windows-audio/Cargo.toml');
  assert.match(cargo, /\[target\.'cfg\(windows\)'\.dependencies\]/);
  assert.match(cargo, /wasapi\s*=\s*"=0\.23\.0"/);
});

test('Windows audio crate remains safe Rust at the crate boundary', async () => {
  const source = await read('crates/voxveil-windows-audio/src/lib.rs');
  assert.match(source, /#!\[forbid\(unsafe_code\)\]/);
});

test('Tauri processing commands delegate enable and vocal level to runtime controller', async () => {
  const source = await read('tauri/app/commands.rs');
  assert.match(source, /ProcessingController/);
  assert.match(source, /controller\.set_enabled/);
  assert.match(source, /controller\.set_vocal_level/);
});

test('current Windows backend is honest about all-output-only capability', async () => {
  const dto = await read('tauri/app/dto.rs');
  const commands = await read('tauri/app/commands.rs');
  const home = await read('ui/features/home/HomeScreen.tsx');
  assert.match(dto, /per_app_processing_available/);
  assert.match(commands, /mode == ProcessingMode::PerApp && !snapshot\.per_app_available/);
  assert.match(home, /disabled: !state\.perAppProcessingAvailable/);
});

test('manual Windows build stages the real componentized APO package', async () => {
  const build = await read('scripts/windows/build-windows.ps1');
  const extension = await read('native/windows/package/VoxveilApoExtension.inf.template');
  const install = await read('scripts/windows/install-system-audio-component.ps1');

  assert.match(build, /VoxveilApo\.dll/);
  assert.match(build, /VoxveilControl\.dll/);
  assert.match(build, /voxveil-control\.exe/);
  assert.match(build, /VoxveilApo\.inf/);
  assert.match(build, /VoxveilApoExtension\.inf\.template/);
  assert.match(extension, /AddComponent/i);
  assert.match(install, /pnputil/i);
  assert.doesNotMatch(install, /Root\\Sysvad_ComponentizedAudioSample/);
});

test('Windows readiness requires the APO to be loaded on the active render endpoint', async () => {
  const device = await read('crates/voxveil-windows-audio/src/device.rs');
  const backend = await read('crates/voxveil-windows-audio/src/relay.rs');

  assert.match(device, /loaded_instances == 0/);
  assert.match(device, /AudioDG has not loaded/);
  assert.match(backend, /run_control\(&control, &\["status"\]\)/);
  assert.match(backend, /strip_prefix\("loaded="\)/);
  assert.doesNotMatch(backend, /virtual_render_device/);
  assert.doesNotMatch(backend, /initialize_client\(/);
});

test('WASAPI COM initialization converts HRESULT before Rust error mapping', async () => {
  const source = await read('crates/voxveil-windows-audio/src/relay.rs');
  const initializers = source.match(/wasapi::initialize_mta\(\)\s*\.ok\(\)\s*\.map_err/g) ?? [];
  assert.equal(initializers.length, 1);
  assert.doesNotMatch(source, /initialize_mta\(\)\.map_err/);
});
