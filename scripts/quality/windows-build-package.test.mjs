import assert from 'node:assert/strict';
import test from 'node:test';
import { readFile } from 'node:fs/promises';

const buildScript = await readFile(
  new URL('../windows/build-windows.ps1', import.meta.url),
  'utf8',
);

test('Windows package checksum paths work in Windows PowerShell 5.1', () => {
  assert.doesNotMatch(buildScript, /\[IO\.Path\]::GetRelativePath\s*\(/);
  assert.match(buildScript, /function Get-RelativePackagePath/);
  assert.match(buildScript, /StringComparison\]::OrdinalIgnoreCase/);
});

test('Windows package fails if a required system-audio script is missing', () => {
  const start = buildScript.indexOf('foreach ($script in @(');
  const end = buildScript.indexOf('if ($signedApoDir) {', start);
  const staging = start >= 0 && end > start ? buildScript.slice(start, end) : '';

  for (const script of [
    'discover-system-audio-endpoints.ps1',
    'new-apo-extension-inf.ps1',
    'install-system-audio-component.ps1',
    'uninstall-system-audio-component.ps1',
    'probe-apo-capx.ps1',
    'install-staged-virtual-driver.ps1',
    'uninstall-staged-virtual-driver.ps1',
  ]) {
    assert.match(staging, new RegExp(script.replaceAll('.', '\\.')));
  }

  assert.match(staging, /if \(-not \(Test-Path \$source -PathType Leaf\)\) \{\s*throw/i);
  assert.match(staging, /Copy-Item \$source \$systemAudio/);
  assert.doesNotMatch(staging, /if \(Test-Path \$source -PathType Leaf\) \{\s*Copy-Item/i);
});

test('Windows package starts from a clean output tree before signed virtual-driver staging', () => {
  const removeOutput = buildScript.indexOf('Remove-Item $output -Recurse -Force -ErrorAction SilentlyContinue');
  const recreateSystemAudio = buildScript.indexOf('New-Item -ItemType Directory -Force -Path $systemAudio', removeOutput);
  const signedDriverStage = buildScript.indexOf("stage-signed-virtual-driver.ps1", recreateSystemAudio);

  assert.ok(removeOutput >= 0, 'build must delete the previous package output tree');
  assert.ok(recreateSystemAudio > removeOutput, 'system-audio staging must be recreated only after output cleanup');
  assert.ok(signedDriverStage > recreateSystemAudio, 'signed driver staging must run only inside the newly cleaned package tree');
});


test('Windows package output cleanup is preflighted against repository and signed-input paths', () => {
  const outputResolve = buildScript.indexOf('$output = [IO.Path]::GetFullPath($OutputDirectory)');
  const removeOutput = buildScript.indexOf('Remove-Item $output -Recurse -Force -ErrorAction SilentlyContinue');
  assert.ok(outputResolve >= 0 && removeOutput > outputResolve);

  const preflight = buildScript.slice(outputResolve, removeOutput);
  assert.match(preflight, /Assert-SafeOutputDirectory/i);
  assert.match(preflight, /\$repoPath/i);
  assert.match(preflight, /\$distRoot/i);
  assert.match(preflight, /SignedInputDirectories\s+@\(\$signedApoDir,\s*\$signedDriverDir\)/i);
  assert.match(buildScript.slice(0, outputResolve), /\$signedApoDir\s*=\s*\$env:VOXVEIL_SIGNED_APO_DIR/i);
  assert.match(preflight, /\$signedDriverDir\s*=\s*\$env:VOXVEIL_SIGNED_DRIVER_DIR/i);
  assert.match(buildScript, /must not overlap signed input directory/i);
});

test('Windows package only allows output below the repository dist/windows-x64 root', () => {
  assert.match(buildScript, /function\s+Assert-SafeOutputDirectory/i);
  assert.match(buildScript, /output directory must be below the repository dist\\windows-x64 tree/i);
  assert.match(buildScript, /\$outputFull\s*-ieq\s*\$distFull/i);
  assert.match(buildScript, /-not\s*\(Test-DirectoryContains\s+\$distFull\s+\$outputFull\)/i);
  assert.match(buildScript, /GetPathRoot/i);
});


test('Windows package output rejects junction or symlink ancestors before cleanup', () => {
  assert.match(buildScript, /function\s+Assert-NoReparsePointInPath/i);
  assert.match(buildScript, /Get-Item\s+-LiteralPath\s+\$current\s+-Force/i);
  assert.match(buildScript, /FileAttributes\]::ReparsePoint/i);
  const reparseCheck = buildScript.indexOf('Assert-NoReparsePointInPath -Path $output -Boundary $repoPath');
  const removeOutput = buildScript.indexOf('Remove-Item $output -Recurse -Force -ErrorAction SilentlyContinue');
  assert.ok(reparseCheck >= 0 && removeOutput > reparseCheck, 'reparse-point preflight must precede recursive output cleanup');
});


test('Windows build embeds privileged helper hashes before compiling Tauri', () => {
  const tauriBuild = buildScript.indexOf('npm run tauri -- build --no-bundle');
  assert.ok(tauriBuild >= 0);
  for (const name of [
    'VOXVEIL_DISCOVERY_SHA256',
    'VOXVEIL_CONTROL_SHA256',
    'VOXVEIL_CONTROL_DLL_SHA256',
  ]) {
    const assignment = buildScript.indexOf(`$env:${name} =`);
    assert.ok(assignment >= 0 && assignment < tauriBuild, `${name} must be embedded before Tauri build`);
  }
});


test('Windows package rechecks final privileged helper bytes against Tauri trust anchors', () => {
  const tauriBuild = buildScript.indexOf('npm run tauri -- build --no-bundle');
  const packageHashes = buildScript.indexOf('$hashFiles = Get-ChildItem $output -Recurse -File');
  assert.ok(tauriBuild >= 0 && packageHashes > tauriBuild);

  const finalPackage = buildScript.slice(tauriBuild, packageHashes);
  assert.match(finalPackage, /trustedInstallerSha256/i);
  assert.match(finalPackage, /VOXVEIL_DISCOVERY_SHA256/i);
  assert.match(finalPackage, /VOXVEIL_CONTROL_SHA256/i);
  assert.match(finalPackage, /VOXVEIL_CONTROL_DLL_SHA256/i);
  assert.match(finalPackage, /Get-Sha256Hex/i);
  assert.match(finalPackage, /Packaged trusted file changed after Tauri trust anchors were compiled/i);
});


test('Windows build embeds exact verified production APO package hashes before compiling Tauri', () => {
  const tauriBuild = buildScript.indexOf('npm run tauri -- build --no-bundle');
  assert.ok(tauriBuild >= 0);
  assert.match(buildScript.slice(0, tauriBuild), /verify-signed-apo-package\.ps1/i);
  for (const name of [
    'VOXVEIL_APO_INF_SHA256',
    'VOXVEIL_APO_DLL_SHA256',
    'VOXVEIL_APO_CATALOG_SHA256',
    'VOXVEIL_APO_EXTENSION_INF_SHA256',
    'VOXVEIL_APO_EXTENSION_CATALOG_SHA256',
  ]) {
    const assignment = buildScript.indexOf(`$env:${name} =`);
    assert.ok(assignment >= 0 && assignment < tauriBuild, `${name} must be set before Tauri build`);
  }
});


test('Windows package rechecks final signed APO files against embedded build-time hashes', () => {
  const tauriBuild = buildScript.indexOf('npm run tauri -- build --no-bundle');
  const packageHashes = buildScript.indexOf('$hashFiles = Get-ChildItem $output -Recurse -File');
  const finalPackage = buildScript.slice(tauriBuild, packageHashes);
  for (const [file, envName] of [
    ['VoxveilApo.inf', 'VOXVEIL_APO_INF_SHA256'],
    ['VoxveilApo.dll', 'VOXVEIL_APO_DLL_SHA256'],
    ['VoxveilApo.cat', 'VOXVEIL_APO_CATALOG_SHA256'],
    ['VoxveilApoExtension.inf', 'VOXVEIL_APO_EXTENSION_INF_SHA256'],
    ['VoxveilApoExtension.cat', 'VOXVEIL_APO_EXTENSION_CATALOG_SHA256'],
  ]) {
    assert.match(finalPackage, new RegExp(file.replaceAll('.', '\\.')));
    assert.match(finalPackage, new RegExp(envName));
  }
  assert.match(finalPackage, /Packaged trusted file changed after Tauri trust anchors were compiled/i);
});

test('Windows package build pins MSBuild and WDK roots to OS-known Program Files', () => {
  const text = buildScript;
  assert.match(text, /GetFolderPath\(\[Environment\+SpecialFolder\]::ProgramFilesX86\)/i);
  assert.match(text, /Microsoft Visual Studio\\Installer\\vswhere\.exe/i);
  assert.match(text, /Windows Kits\\10\\Include/i);
  assert.doesNotMatch(text, /\$env:ProgramFiles\(x86\)/i);
  assert.doesNotMatch(text, /Get-Command\s+msbuild\.exe/i);
});
