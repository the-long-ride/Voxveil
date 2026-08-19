import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const read = (file) => readFile(new URL(`../../${file}`, import.meta.url), 'utf8');

test('workspace includes isolated Windows audio crate', async () => {
  const cargo = await read('Cargo.toml');
  assert.match(cargo, /crates\/voxveil-windows-audio/);
});

test('Windows audio crate keeps exact approved dependencies only on Windows', async () => {
  const cargo = await read('crates/voxveil-windows-audio/Cargo.toml');
  assert.match(cargo, /\[target\.'cfg\(windows\)'\.dependencies\]/);
  assert.match(cargo, /wasapi\s*=\s*"=0\.23\.0"/);
  assert.match(cargo, /serde_json\s*=\s*"=1\.0\.151"/);
  assert.match(cargo, /windows\s*=\s*\{\s*version\s*=\s*"=0\.62\.2"/);
});

test('Windows audio crate confines unsafe code to audited FFI modules', async () => {
  const lib = await read('crates/voxveil-windows-audio/src/lib.rs');
  const topology = await read('crates/voxveil-windows-audio/src/topology.rs');
  const deviceInterfaces = await read('crates/voxveil-windows-audio/src/device_interfaces.rs');

  assert.match(lib, /#!\[deny\(unsafe_code\)\]/);
  assert.doesNotMatch(lib, /#!\[allow\(unsafe_code\)\]/);
  assert.match(lib, /#\[allow\(unsafe_code\)\]\s*mod topology;/);
  assert.match(deviceInterfaces, /#\[allow\(unsafe_code\)\]\s*mod windows_runtime/);
  assert.match(topology, /CoCreateInstance|CoInitializeEx/);
  assert.match(deviceInterfaces, /SetupDiGetClassDevsW|SetupDiEnumDeviceInterfaces/);
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

test('manual Windows build stages componentized APO and endpoint discovery', async () => {
  const build = await read('scripts/windows/build-windows.ps1');
  const extension = await read('native/windows/package/VoxveilApoExtension.inf.template');
  const install = await read('scripts/windows/install-system-audio-component.ps1');

  assert.match(build, /VoxveilApo\.dll/);
  assert.match(build, /VoxveilControl\.dll/);
  assert.match(build, /voxveil-control\.exe/);
  assert.match(build, /discover-system-audio-endpoints\.ps1/);
  assert.match(extension, /AddComponent/i);
  assert.match(install, /EndpointDescriptor/);
  assert.match(install, /pnputil/i);
  assert.doesNotMatch(install, /Root\\Sysvad_ComponentizedAudioSample/);
});

test('system audio discovery fails closed instead of guessing topology', async () => {
  const discovery = await read('crates/voxveil-windows-audio/src/discovery.rs');
  const helper = await read('scripts/windows/discover-system-audio-endpoints.ps1');
  const deviceInterfaces = await read('crates/voxveil-windows-audio/src/device_interfaces.rs');
  assert.match(discovery, /SystemAudioEndpointStatus::Ambiguous/);
  assert.match(discovery, /select_topology_candidate/);
  assert.match(discovery, /RuntimeBindingKind::Ambiguous/);
  assert.match(helper, /Voxveil will not guess/);
  assert.match(helper, /runtimeDeviceId/);
  assert.match(helper, /runtimeAliasMatch/);
  assert.match(helper, /DEVPKEY_Device_HardwareIds/);
  assert.match(helper, /DEVPKEY_Device_DriverInfPath/);
  assert.match(deviceInterfaces, /const KSCATEGORY_TOPOLOGY/);
  assert.match(deviceInterfaces, /0xdda54a40_1e4c_11d1_a050_405705c10000/i);
});

test('runtime topology binding can source driver metadata from an anchored ancestor', async () => {
  const discovery = await read('crates/voxveil-windows-audio/src/discovery.rs');
  const helper = await read('scripts/windows/discover-system-audio-endpoints.ps1');
  const installer = await read('scripts/windows/install-system-audio-component.ps1');
  const systemAudio = await read('tauri/app/system_audio.rs');

  assert.match(discovery, /binding_pnp_instance_id/);
  assert.match(helper, /bindingPnpInstanceId/);
  assert.match(helper, /Resolve-ParentAudioDevice\s+\$runtimeDeviceId/);
  assert.match(systemAudio, /binding_pnp_instance_id/);
  assert.match(installer, /bindingPnpInstanceId/);
  assert.match(installer, /runtimeDeviceId\s*=\s*\[string\]\$descriptor\.bindingPnpInstanceId/);
});

test('browser install flow uses opaque endpoint id and never raw driver identifiers', async () => {
  const home = await read('ui/features/home/HomeScreen.tsx');
  const model = await read('ui/app/useVoxveilState.ts');
  const client = await read('ui/lib/tauri.ts');
  const types = await read('ui/lib/types.ts');
  const systemAudio = await read('tauri/app/system_audio.rs');
  const dto = await read('tauri/app/dto.rs');
  const tauri = await read('tauri/lib.rs');

  assert.match(client, /installSystemAudioComponent: \(endpointId: string\)/);
  assert.match(client, /list_system_audio_endpoints/);
  assert.match(systemAudio, /endpoint_id: String/);
  assert.match(systemAudio, /select_installable_endpoint/);
  assert.match(systemAudio, /EndpointInstallDescriptor/);
  assert.match(tauri, /app::system_audio::install_system_audio_component/);
  for (const source of [home, model, client, types, dto]) {
    assert.doesNotMatch(source, /HardwareId|ReferenceString|hardwareIds|topologyReference/);
  }
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
