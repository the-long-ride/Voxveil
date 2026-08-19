import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const root = process.cwd();
const apoRoot = path.join(root, 'native', 'windows', 'apo');

function read(name) {
  return fs.readFileSync(path.join(apoRoot, name), 'utf8');
}

function readRepo(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

function assertNoTestModeCommands(text) {
  assert.doesNotMatch(text, /bcdedit(?:\.exe)?[^\r\n]*testsigning/i);
  assert.doesNotMatch(text, /\/set\s+testsigning\s+(?:on|yes|1)/i);
}

test('Windows 11 APO package is componentized and catalog-backed', () => {
  const inf = read('VoxveilApo.inf');
  assert.match(inf, /Class\s*=\s*AudioProcessingObject/i);
  assert.match(inf, /5989fce8-9cd0-467d-8a6a-5419e31529d4/i);
  assert.match(inf, /CatalogFile\s*=\s*VoxveilApo\.cat/i);
  assert.match(inf, /SWC\\VEN_VOXV&CID_APO/i);
  assert.match(inf, /%13%\\VoxveilApo\.dll/i);
  assert.match(inf, /\[ApoComponent_Install\.Services\][\s\S]*AddService\s*=\s*,2/i);
  assert.match(inf, /\[SignatureAttributes\][\s\S]*VoxveilApo\.dll\s*=\s*SignatureAttributes\.PETrust/i);
  assert.match(inf, /\[SignatureAttributes\.PETrust\][\s\S]*PETrust\s*=\s*true/i);
});

test('Extension INF generator targets exactly one render device and adds the APO component', () => {
  const extension = read('extension.ps1');
  assert.match(extension, /Targets\.Count\s*-ne\s*1/i);
  assert.match(extension, /CatalogFile=VoxveilExtension\.cat/i);
  assert.match(extension, /AddComponent=VoxveilApo/i);
  assert.match(extension, /ComponentIDs=VEN_VOXV&CID_APO/i);
  assert.match(extension, /KSCATEGORY_AUDIO/i);
  assert.match(extension, /KSCATEGORY_TOPOLOGY/i);
  assert.match(extension, /PKEY_CompositeFX_EndpointEffectClsid/i);
  assert.match(extension, /PKEY_EFX_ProcessingModes_Supported_For_Streaming/i);
});

test('target helper discovers only the current default render endpoint', () => {
  const source = read('target_discovery.cpp');
  const project = read('VoxveilApoTarget.vcxproj');
  assert.match(source, /IMMDeviceEnumerator/);
  assert.match(source, /GetDefaultAudioEndpoint/);
  assert.match(source, /PKEY_Device_ContainerId/);
  assert.match(source, /DEVPKEY_Device_ContainerId/);
  assert.match(source, /KSCATEGORY_RENDER|65e8773e/i);
  assert.match(source, /KSCATEGORY_TOPOLOGY|dda54a40/i);
  assert.doesNotMatch(source, /--install-fx/);
  assert.match(source, /--cleanup-runtime/);
  assert.doesNotMatch(source, /SetupDiCreateDeviceInterfaceRegKey/);
  assert.match(project, /mmdevapi\.lib|ole32\.lib/i);
  assert.match(project, /setupapi\.lib/i);
});

test('installer generates signed catalogs and installs both PnP packages without Test Mode', () => {
  const install = read('install.ps1');
  assert.match(install, /New-FileCatalog/i);
  assert.match(install, /Set-AuthenticodeSignature/i);
  assert.match(install, /New-SelfSignedCertificate/i);
  assert.match(install, /TrustedPublisher/i);
  assert.match(install, /VoxveilApo\.inf/i);
  assert.match(install, /New-VoxveilExtensionInf/i);
  assert.match(install, /pnputil(?:\.exe)?[^\r\n]*\/add-driver/i);
  assert.match(install, /\/restart-device/i);
  assert.match(install, /--cleanup-runtime/i);
  assert.doesNotMatch(install, /--install-fx/i);
  assert.doesNotMatch(install, /MMDevices\\Audio\\Render/i);
  assertNoTestModeCommands(install);
});

test('uninstaller removes Voxveil driver-store packages and development trust', () => {
  const uninstall = read('uninstall.ps1');
  assert.match(uninstall, /Get-WindowsDriver/i);
  assert.match(uninstall, /pnputil(?:\.exe)?[^\r\n]*\/delete-driver/i);
  assert.match(uninstall, /TrustedPublisher/i);
  assert.match(uninstall, /--cleanup-runtime/i);
  assert.doesNotMatch(uninstall, /--remove-fx/i);
  assertNoTestModeCommands(uninstall);
});

test('standalone EXE embeds the complete componentized installer payload', () => {
  const systemAudio = readRepo('tauri/app/system_audio.rs');
  assert.match(systemAudio, /include_bytes!\([^)]*VoxveilApo\.dll/i);
  assert.match(systemAudio, /include_bytes!\([^)]*VoxveilApoCheck\.exe/i);
  assert.match(systemAudio, /include_bytes!\([^)]*VoxveilApoTarget\.exe/i);
  assert.match(systemAudio, /include_str!\([^)]*VoxveilApo\.inf/i);
  assert.match(systemAudio, /include_str!\([^)]*targets\.ps1/i);
  assert.match(systemAudio, /include_str!\([^)]*extension\.ps1/i);
  assert.match(systemAudio, /include_str!\([^)]*install\.ps1/i);
  assert.match(systemAudio, /include_str!\([^)]*uninstall\.ps1/i);
});

test('APO registration metadata agrees with the native APO implementation', () => {
  const inf = read('VoxveilApo.inf');
  const source = read('voxveil_apo.cpp');
  assert.match(inf, /"Flags",0x00010001,0x0000000e/i);
  assert.match(source, /APO_FLAG_DEFAULT/);
  assert.doesNotMatch(source, /APO_FLAG_INPLACE\s*\|\s*APO_FLAG_DEFAULT/);
});

test('backend readiness requires a recent APO processing heartbeat', () => {
  const backend = readRepo('crates/voxveil-windows-audio/src/apo.rs');
  const control = read('control_state.cpp');
  const apo = read('voxveil_apo.cpp');
  assert.match(backend, /apo-runtime\.bin/i);
  assert.match(backend, /process_count|processed/i);
  assert.match(control, /apo-runtime\.bin/i);
  assert.match(control, /process_count/i);
  assert.match(apo, /note_process/i);
});