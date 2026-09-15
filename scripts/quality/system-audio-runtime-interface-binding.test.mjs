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

test('legacy development APO attachment opens opaque SetupAPI interface registry keys', () => {
  assert.match(control, /SetupDiOpenDeviceInterfaceW/);
  assert.match(control, /SetupDiOpenDeviceInterfaceRegKey/);
  assert.match(control, /kCompositeSfx/);
  assert.match(control, /\{D04E05A6-594B-4fb6-A80D-01AF5EED7D1D\},13/i);
  assert.match(control, /attach-effects/);
  assert.match(control, /detach-effects/);
  assert.doesNotMatch(control, /ReferenceString|reference string/i);
});

test('production CAPX uses signed AddInterface while runtime registry attachment is test-sign only', () => {
  assert.match(extensionTemplate, /VOXVEIL_APO_CONTEXT/);
  assert.match(generator, /AddInterface\s*=\s*%KSCATEGORY_AUDIO%/i);
  assert.match(generator, /FX\\0\\%VOXVEIL_APO_CONTEXT%/i);
  assert.match(installer, /CAPX production/i);
  assert.match(installer, /\$TestSign\s+-and\s+\$runtimeBound/);
  assert.match(installer, /attach-effects/);
  assert.doesNotMatch(generator, /Mandatory\s*=\s*\$true[^\r\n]*\r?\n[^\r\n]*\[string\]\$ReferenceString/);
});
