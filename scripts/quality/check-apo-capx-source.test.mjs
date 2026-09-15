import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(path, 'utf8');

test('APO project stays on the approved C++17 Windows driver toolchain', () => {
  const project = read('native/windows/apo/VoxveilApo.vcxproj');
  assert.match(project, /<LanguageStandard>stdcpp17<\/LanguageStandard>/);
  assert.match(project, /<PlatformToolset>WindowsApplicationForDrivers10\.0<\/PlatformToolset>/);
  assert.doesNotMatch(project, /stdcpp20|stdcpplatest/i);
});

test('APO constructor never claims a loaded processing instance', () => {
  const text = read('native/windows/apo/VoxveilApo.cpp');
  const constructor = text.match(/CVoxveilApo::CVoxveilApo\(\) noexcept[\s\S]*?\n}\n\nCVoxveilApo::~CVoxveilApo/);
  assert.ok(constructor, 'APO constructor block not found');
  assert.doesNotMatch(constructor[0], /loadedInstances|capxInstances/);
});

test('APO initialization owns discovery-aware readiness counting', () => {
  const text = read('native/windows/apo/VoxveilApo.cpp');
  assert.match(text, /ShouldCountLoadedInstance\(flavor, initializeForDiscoveryOnly_\)/);
  assert.match(text, /InterlockedIncrement\(&state_->loadedInstances\)/);
  assert.match(text, /InterlockedIncrement\(&state_->capxInstances\)/);
});

test('real-time DSP requires both independent enable gates', () => {
  const text = read('native/windows/apo/VoxveilApo.cpp');
  assert.match(text, /state_->enabled/);
  assert.match(text, /state_->systemEffectEnabled/);
  assert.match(text, /ShouldProcess\(appEnabled, systemEffectEnabled, vocal\)/);
  assert.doesNotMatch(text, /RETURN_IF_FAILED|RETURN_HR_IF|wil::/);
});

test('APO uses non-throwing SRW lock ownership for effect-event state', () => {
  const header = read('native/windows/apo/VoxveilApo.h');
  const source = read('native/windows/apo/VoxveilApo.cpp');
  assert.match(header, /SRWLOCK\s+effectsLock_\s*=\s*SRWLOCK_INIT/);
  assert.doesNotMatch(header, /CComAutoCriticalSection|CComCritSecLock/);
  assert.match(source, /AcquireSRWLockExclusive\(&effectsLock_\)/);
  assert.match(source, /ReleaseSRWLockExclusive\(&effectsLock_\)/);
});

test('CAPX default store is read-only while user and volatile stores are writable', () => {
  const text = read('native/windows/apo/VoxveilApo.cpp');
  assert.match(text, /OpenDefaultPropertyStore\(STGM_READ,\s*&rawStore\)/);
  assert.match(text, /OpenUserPropertyStore\(STGM_READWRITE,\s*&rawStore\)/);
  assert.match(text, /OpenVolatilePropertyStore\(STGM_READWRITE,\s*&rawStore\)/);
});

test('shared-state initialization publishes ABI only after defaults', () => {
  const text = read('native/windows/apo/VoxveilSharedState.h');
  const enabledIndex = text.indexOf('InterlockedExchange(&state->systemEffectEnabled, 1)');
  const publishIndex = text.indexOf('InterlockedExchange(&state->abi, kSharedStateAbi)');
  assert.ok(enabledIndex >= 0, 'system-effect default initialization missing');
  assert.ok(publishIndex > enabledIndex, 'ABI must be published after state defaults');
  assert.match(text, /ERROR_ALREADY_EXISTS/);
});

test('fixed C++ CAPX identities match the extension package identity', () => {
  const ids = read('native/windows/apo/VoxveilApoIds.h');
  const inf = read('native/windows/package/VoxveilApoExtension.inf.template');
  assert.match(ids, /b9fd554e.*8f72.*4b20.*9a.*b1.*13.*f8.*e8.*bf.*dd.*02/is);
  assert.match(ids, /63e268ce.*4cbc.*48e0.*be.*b6.*55.*10.*33.*16.*f4.*77/is);
  assert.match(inf, /\{63E268CE-4CBC-48E0-BEB6-55103316F477\}/i);
});
