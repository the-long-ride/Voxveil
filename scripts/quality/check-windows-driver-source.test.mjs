import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('virtual driver source uses the fixed Voxveil GUID identities', () => {
  const text = read('native/windows/driver/voxveil_ids.h');
  assert.match(text, /79e4e58c.*9714.*44e8.*ac.*a1.*42.*6f.*24.*b7.*a1.*e9/is);
  assert.match(text, /3f67f54b.*dcf8.*4cb9.*9e.*0e.*79.*a1.*3c.*fd.*04.*6b/is);
});

test('SysVAD-derived Voxveil endpoint tables retain in-file provenance', () => {
  for (const path of [
    'native/windows/driver/voxveil_endpoint.h',
    'native/windows/driver/voxveil_topology.h',
    'native/windows/driver/voxveil_wavert.h',
  ]) {
    const text = read(path);
    assert.match(text, /Microsoft Windows Driver Samples SysVAD/i, `${path} is missing SysVAD provenance`);
    assert.match(text, /MS-PL/i, `${path} is missing the upstream license marker`);
  }
});

test('endpoint definition exposes exactly one render pair and zero capture pairs', () => {
  const text = read('native/windows/driver/voxveil_endpoint.h');
  assert.match(text, /g_RenderEndpoints\s*\[\]\s*=\s*\{\s*&VoxveilMiniports\s*\}/s);
  assert.match(text, /g_cCaptureEndpoints\s+0\b/);
  assert.match(text, /static_assert\s*\(\s*g_cRenderEndpoints\s*==\s*1/i);
  assert.match(text, /static_assert\s*\(\s*g_cCaptureEndpoints\s*==\s*0/i);
  assert.doesNotMatch(text, /MicArray|Hdmi|Spdif|Bluetooth|UsbHs|A2dp/i);
});

test('WaveRT table has only host render and bridge pins with float32 stereo', () => {
  const text = read('native/windows/driver/voxveil_wavert.h');
  assert.match(text, /SystemRenderPin/);
  assert.match(text, /BridgePin/);
  assert.match(text, /KSDATAFORMAT_SUBTYPE_IEEE_FLOAT/);
  assert.match(text, /WAVE_FORMAT_EXTENSIBLE/);
  assert.match(text, /\b32\b/);
  assert.doesNotMatch(text, /RenderLoopbackPin|OffloadRenderPin|AudioEngine|KSPROPSETID_OffloadPin/i);
});

test('driver project references only pinned generic SysVAD source plus Voxveil-owned endpoint tables', () => {
  const text = read('native/windows/driver/VoxveilVirtualAudio.vcxproj');
  assert.match(text, /third_party\\microsoft\\windows-driver-samples\\audio\\sysvad/i);
  assert.match(text, /EndpointsCommon\\minwavert\.cpp/i);
  assert.match(text, /EndpointsCommon\\minwavertstream\.cpp/i);
  assert.match(text, /EndpointsCommon\\mintopo\.cpp/i);
  assert.doesNotMatch(text, /A2dp|Bth|UsbHs|MicArray|KeywordDetector|SwapAPO|DelayAPO/i);
});

test('directly compiled pinned EndpointsCommon sources preserve upstream warning compatibility', () => {
  const text = read('native/windows/driver/VoxveilVirtualAudio.vcxproj');
  assert.match(text, /<TreatWarningAsError>true<\/TreatWarningAsError>/i);
  assert.match(text, /<DisableSpecificWarnings>[^<]*\b4595\b[^<]*%\(DisableSpecificWarnings\)[^<]*<\/DisableSpecificWarnings>/i);
});

test('virtual driver project carries Voxveil sound-driver version resources and filters', () => {
  const project = read('native/windows/driver/VoxveilVirtualAudio.vcxproj');
  const resource = read('native/windows/driver/VoxveilVirtualAudio.rc');
  const filters = read('native/windows/driver/VoxveilVirtualAudio.vcxproj.filters');
  assert.match(project, /<ResourceCompile Include="VoxveilVirtualAudio\.rc"\s*\/>/i);
  assert.match(resource, /VFT2_DRV_SOUND/);
  assert.match(resource, /Voxveil Virtual Audio Driver/);
  assert.match(resource, /VoxveilVirtualAudio\.sys/);
  assert.match(filters, /VoxveilVirtualAudio\.rc/i);
  assert.match(filters, /voxveil_endpoint\.h/i);
});

test('WDK project disables automatic driver signing for every build configuration', () => {
  const text = read('native/windows/driver/VoxveilVirtualAudio.vcxproj');
  assert.equal((text.match(/<SignMode>Off<\/SignMode>/g) ?? []).length, 4);
  assert.doesNotMatch(text, /TestSign|Test Certificate|\.pfx|\.cer/i);
});

test('driver substitutes a no-op save-data implementation so audio is never persisted', () => {
  const project = read('native/windows/driver/VoxveilVirtualAudio.vcxproj');
  const saveData = read('native/windows/driver/voxveil_savedata.cpp');
  assert.match(project, /voxveil_savedata\.cpp/i);
  assert.doesNotMatch(project, /\\savedata\.cpp/i);
  assert.doesNotMatch(saveData, /ZwCreateFile|ZwWriteFile|CreateFile|WriteFile/i);
});

test('driver replaces the SysVAD sample tone generator with an inert Voxveil implementation', () => {
  const project = read('native/windows/driver/VoxveilVirtualAudio.vcxproj');
  const toneGenerator = read('native/windows/driver/voxveil_tonegenerator.cpp');
  assert.match(project, /voxveil_tonegenerator\.cpp/i);
  assert.doesNotMatch(project, /third_party[^\r\n]*tonegenerator\.cpp/i);
  assert.match(toneGenerator, /RtlZeroMemory\s*\(/);
  assert.doesNotMatch(toneGenerator, /sin\s*\(|cos\s*\(|M_PI|ToneAmplitude\s*\*/i);
});

test('production INF exposes the fixed Voxveil interface and disables sample diagnostics', () => {
  const text = read('native/windows/driver/package/VoxveilVirtualAudio.inf');
  assert.match(text, /79E4E58C-9714-44E8-ACA1-426F24B7A1E9/i);
  assert.match(text, /DoNotCreateDataFiles,0x00010001,1/i);
  assert.match(text, /DisableToneGenerator,0x00010001,1/i);
});

test('SysVAD importer verifies the exact fetched commit and subtree before materializing source', () => {
  const text = read('scripts/windows/import-sysvad-source.ps1');
  assert.match(text, /git(?:\.exe)?['"]?\s+fetch|&\s*\$git\s+.*fetch/is);
  assert.match(text, /FETCH_HEAD:audio\/sysvad/i);
  assert.match(text, /6fa502f5bfb3de1395a6c9ffe71e322fd9e28926/i);
  assert.match(text, /67d81f217bc01edf7a4320e4911c11065635acfa/i);
  assert.match(text, /git(?:\.exe)?['"]?\s+archive|&\s*\$git\s+.*archive/is);
  assert.doesNotMatch(text, /Invoke-WebRequest/i);
});

test('materialized SysVAD source stays generated-only and provenance documents the build-time contract', () => {
  const gitignore = read('.gitignore');
  const provenance = read('third_party/microsoft/windows-driver-samples/README.voxveil.md');
  assert.match(gitignore, /^third_party\/microsoft\/windows-driver-samples\/audio\/sysvad\/$/m);
  assert.match(provenance, /materialized at build time/i);
  assert.match(provenance, /SOURCE_REVISION/i);
  assert.match(provenance, /SYSVAD_TREE_SHA/i);
  assert.doesNotMatch(provenance, /checked-in snapshot/i);
});

test('virtual driver build materializes the verified pinned SysVAD source before compiling', () => {
  const text = read('scripts/windows/build-virtual-driver.ps1');
  assert.match(text, /import-sysvad-source\.ps1/i);
  assert.match(text, /-Force/i);
  assert.match(text, /SYSVAD_TREE_SHA/i);
  assert.match(text, /6fa502f5bfb3de1395a6c9ffe71e322fd9e28926/i);
  assert.match(text, /SysVAD tree.*drift/i);
});

test('virtual driver build stages only the requested architecture and configuration output', () => {
  const text = read('scripts/windows/build-virtual-driver.ps1');
  assert.match(text, /build\\\$Architecture\\\$Configuration/i);
  assert.match(text, /Join-Path\s+\$BuildOutput\s+'VoxveilVirtualAudio\.sys'/i);
  assert.match(text, /Join-Path\s+\$BuildOutput\s+'VoxveilVirtualAudio\.pdb'/i);
  assert.doesNotMatch(text, /Get-ChildItem\s+\$DriverRoot\s+-Recurse\s+-File\s+-Filter\s+'VoxveilVirtualAudio\.(?:sys|pdb)'/i);
});
