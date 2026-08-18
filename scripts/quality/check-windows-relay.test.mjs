import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const read = (file) => readFile(new URL(`../../${file}`, import.meta.url), 'utf8');

test('workspace includes isolated Windows audio relay crate', async () => {
  const cargo = await read('Cargo.toml');
  assert.match(cargo, /crates\/voxveil-windows-audio/);
});

test('Windows audio crate uses exact MIT WASAPI dependency only on Windows', async () => {
  const cargo = await read('crates/voxveil-windows-audio/Cargo.toml');
  assert.match(cargo, /\[target\.'cfg\(windows\)'\.dependencies\]/);
  assert.match(cargo, /wasapi\s*=\s*"=0\.23\.0"/);
});

test('Windows audio relay remains safe Rust at the crate boundary', async () => {
  const source = await read('crates/voxveil-windows-audio/src/lib.rs');
  assert.match(source, /#!\[forbid\(unsafe_code\)\]/);
});

test('Tauri processing commands delegate enable and vocal level to runtime controller', async () => {
  const source = await read('tauri/app/commands.rs');
  assert.match(source, /ProcessingController/);
  assert.match(source, /controller\.set_enabled/);
  assert.match(source, /controller\.set_vocal_level/);
});

test('current Windows relay is honest about all-output-only capability', async () => {
  const dto = await read('tauri/app/dto.rs');
  const commands = await read('tauri/app/commands.rs');
  const home = await read('ui/features/home/HomeScreen.tsx');
  assert.match(dto, /per_app_processing_available/);
  assert.match(commands, /mode == ProcessingMode::PerApp && !snapshot\.per_app_available/);
  assert.match(home, /disabled: !state\.perAppProcessingAvailable/);
});

test('portable Windows workflow includes a development system-audio component', async () => {
  const workflow = await read('.github/workflows/windows-portable.yml');
  const install = await read('scripts/windows/install-system-audio-component.ps1');
  assert.match(workflow, /windows-2025-vs2026/);
  assert.match(workflow, /microsoft\/Windows-driver-samples/);
  assert.match(workflow, /microsoft\/setup-msbuild@[0-9a-f]{40}/);
  assert.match(workflow, /Build-Samples\.ps1/);
  assert.match(workflow, /-Samples 'audio\.sysvad','setup\.devcon'/);
  assert.match(workflow, /-NtTargetVersion '10\.0\.28000'/);
  assert.match(workflow, /system-audio/);
  assert.match(install, /Root\\Sysvad_ComponentizedAudioSample/);
  assert.match(install, /test-signed/i);
});

test('Windows relay captures the virtual render endpoint through WASAPI loopback', async () => {
  const device = await read('crates/voxveil-windows-audio/src/device.rs');
  const relay = await read('crates/voxveil-windows-audio/src/relay.rs');
  assert.match(device, /virtual audio device \(wdm\) - tablet/);
  assert.doesNotMatch(device, /Voxveil Monitor/);
  assert.match(relay, /virtual_render_device/);
  assert.match(relay, /initialize_client\(&format, &Direction::Capture, &mode\)/);
  assert.doesNotMatch(relay, /is_virtual_capture_name/);
});

test('WASAPI COM initialization converts HRESULT before Rust error mapping', async () => {
  const source = await read('crates/voxveil-windows-audio/src/relay.rs');
  const initializers = source.match(/wasapi::initialize_mta\(\)\s*\.ok\(\)\s*\.map_err/g) ?? [];
  assert.ok(initializers.length >= 2);
  assert.doesNotMatch(source, /initialize_mta\(\)\.map_err/);
});
