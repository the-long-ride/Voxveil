import assert from 'node:assert/strict';
import test from 'node:test';
import { readFile } from 'node:fs/promises';

const [interfaces, discovery, control, installer, extensionTemplate, generator] = await Promise.all([
  readFile(new URL('../../crates/voxveil-windows-audio/src/device_interfaces.rs', import.meta.url), 'utf8'),
  readFile(new URL('../../crates/voxveil-windows-audio/src/discovery.rs', import.meta.url), 'utf8'),
  readFile(new URL('../../native/windows/apo/VoxveilControlCli.cpp', import.meta.url), 'utf8'),
  readFile(new URL('../windows/install-system-audio-component.ps1', import.meta.url), 'utf8'),
  readFile(new URL('../../native/windows/package/VoxveilApoExtension.inf.template', import.meta.url), 'utf8'),
  readFile(new URL('../windows/new-apo-extension-inf.ps1', import.meta.url), 'utf8'),
]);

test('runtime discovery retains exact topology and audio alias paths', () => {
  assert.match(interfaces, /audio_interface_path/);
  assert.match(discovery, /topology_interface_path/);
  assert.match(discovery, /audio_interface_path/);
});

test('runtime APO attachment opens opaque SetupAPI interface registry keys', () => {
  assert.match(control, /SetupDiOpenDeviceInterfaceW/);
  assert.match(control, /SetupDiOpenDeviceInterfaceRegKey/);
  assert.match(control, /kCompositeSfx/);
  assert.match(control, /\{D04E05A6-594B-4fb6-A80D-01AF5EED7D1D\},13/i);
  assert.match(control, /attach-effects/);
  assert.match(control, /detach-effects/);
  assert.doesNotMatch(control, /ReferenceString|reference string/i);
});

test('installer uses runtime interface attachment instead of requiring AddInterface text', () => {
  assert.match(installer, /attach-effects/);
  assert.match(installer, /topologyInterfacePath/);
  assert.match(installer, /audioInterfacePath/);
  assert.doesNotMatch(extensionTemplate, /^\s*AddInterface\s*=/m);
  assert.doesNotMatch(generator, /Mandatory\s*=\s*\$true[^\r\n]*\r?\n[^\r\n]*\[string\]\$ReferenceString/);
});
