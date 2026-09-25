import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const relay = () => [
  'crates/voxveil-windows-audio/src/relay.rs',
  'crates/voxveil-windows-audio/src/relay_support.rs',
  'crates/voxveil-windows-audio/src/relay_control.rs',
].map((path) => readFileSync(path, 'utf8').replace(/\r\n?/g, '\n')).join('\n');

test('missing APO control fails closed when an APO install state exists', () => {
  const text = relay();
  const loaded = text.match(/fn loaded_apo_instances\([\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(loaded, /control_executable_for_installed_apo\(\)\?/);
  assert.match(loaded, /return Ok\(0\)/);
});

test('APO install-state presence check distinguishes only NotFound from existing or unreadable state', () => {
  const text = relay();
  const helper = text.match(/fn apo_install_state_exists\([\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(helper, /install-state\.json/);
  assert.match(helper, /Ok\([^)]*\)\s*=>\s*Ok\(true\)/);
  assert.match(helper, /ErrorKind::NotFound/);
  assert.match(helper, /Err\(error\)/);
  assert.doesNotMatch(helper, /is_file\(\)/);
});

test('APO control resolver errors when installed state exists but the helper is missing', () => {
  const text = relay();
  const resolver = text.match(/fn control_executable_for_installed_apo\([\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(resolver, /control_executable\(\)/);
  assert.match(resolver, /apo_install_state_exists\(\)\?/);
  assert.match(resolver, /control component is unavailable|load state cannot be verified/i);
});

test('live vocal and profile setters use the fail-closed APO control resolver', () => {
  const text = relay();
  const vocal = text.match(/pub fn set_vocal_level\([\s\S]*?\n    \}/)?.[0] ?? '';
  const profile = text.match(/pub fn set_classic_suppression_profile\([\s\S]*?\n    \}/)?.[0] ?? '';

  assert.match(vocal, /control_executable_for_installed_apo\(\)\?/);
  assert.match(profile, /control_executable_for_installed_apo\(\)\?/);
});

test('master disable clears local active state before relay or APO teardown can fail', () => {
  const text = relay();
  const setter = text.slice(text.indexOf('pub fn set_enabled'), text.indexOf('pub fn set_vocal_level'));
  const disable = setter.match(/if !enabled \{[\s\S]*?return Ok\(self\.probe\(\)\);\n        \}/)?.[0] ?? '';

  assert.match(disable, /control_executable_for_installed_apo\(\)\?/);
  assert.doesNotMatch(disable, /control_executable\(\)\.is_some\(\)/);
  assert.ok(
    disable.indexOf('self.enabled = false') >= 0
      && disable.indexOf('self.enabled = false') < disable.indexOf('if let Some(mut relay)'),
    'local active state must be cleared before relay teardown can return an error',
  );
  assert.ok(
    disable.indexOf('self.enabled = false') < disable.indexOf('control_executable_for_installed_apo()?'),
    'local active state must be cleared before APO control resolution can return an error',
  );
});

test('production APO installer requires the control helper for load verification', () => {
  const text = readFileSync('scripts/windows/install-system-audio-component.ps1', 'utf8').replace(/\r\n?/g, '\n');
  const start = text.indexOf('if (Test-Path $control');
  const end = text.indexOf("if ($TestSign) {\n    Write-Host 'Voxveil development/test APO installed", start);
  const verification = start >= 0 && end > start ? text.slice(start, end) : '';

  assert.match(verification, /elseif \(-not \$TestSign\)/);
  assert.match(verification, /throw ['"][^'"\n]*(?:control|load verification|installed-not-loaded)/i);
  assert.match(verification, /else \{[\s\S]*Write-Warning/);
});

test('Windows platform spec forbids relay fallback when installed APO control state cannot be verified', () => {
  const spec = readFileSync('docs/specs/platform/windows.md', 'utf8');

  assert.match(spec, /install-state\.json[\s\S]{0,260}control component/i);
  assert.match(spec, /faulted[\s\S]{0,260}(?:must not|does not|cannot)[\s\S]{0,120}relay fallback|relay fallback[\s\S]{0,260}faulted/i);
});
