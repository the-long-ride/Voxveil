import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const read = (file) => readFile(file, 'utf8');

test('screen headings use the shared ScreenIntro component', async () => {
  const files = [
    'ui/features/home/HomeScreen.tsx',
    'ui/features/apps/AppsScreen.tsx',
    'ui/features/routing/RoutingScreen.tsx',
    'ui/features/engine/EngineScreen.tsx',
    'ui/features/settings/SettingsScreen.tsx',
  ];
  for (const file of files) {
    const source = await read(file);
    assert.match(source, /<ScreenIntro\b/, file);
    assert.doesNotMatch(source, /className="screen-intro"/, file);
  }
});

test('navigation button rendering is centralized', async () => {
  const side = await read('ui/components/SideNavigation.tsx');
  const bottom = await read('ui/components/BottomNavigation.tsx');
  assert.match(side, /<NavigationButtons\b/);
  assert.match(bottom, /<NavigationButtons\b/);
});

test('settings does not access localStorage directly', async () => {
  const settings = await read('ui/features/settings/SettingsScreen.tsx');
  assert.doesNotMatch(settings, /localStorage/);
});

test('feature screens share the exported VoxveilModel type', async () => {
  const files = [
    'ui/features/home/HomeScreen.tsx',
    'ui/features/apps/AppsScreen.tsx',
    'ui/features/routing/RoutingScreen.tsx',
    'ui/features/engine/EngineScreen.tsx',
  ];
  for (const file of files) {
    const source = await read(file);
    assert.match(source, /VoxveilModel/);
    assert.doesNotMatch(source, /ReturnType<typeof useVoxveilState>/);
  }
});

test('native runtime starts from a fail-safe state instead of browser demo data', async () => {
  const hook = await read('ui/app/useVoxveilState.ts');
  assert.match(hook, /SAFE_NATIVE_STATE/);
  assert.match(hook, /native\s*\?\s*SAFE_NATIVE_STATE\s*:\s*PREVIEW_STATE/);
});

test('native command recovery fails closed when backend refresh also fails', async () => {
  const hook = await read('ui/app/useVoxveilState.ts');
  assert.match(hook, /catch\s*\{\s*setState\(SAFE_NATIVE_STATE\);\s*\}/s);
});

test('optimistic state mutation and backend recovery are centralized', async () => {
  const hook = await read('ui/app/useVoxveilState.ts');
  assert.match(hook, /const commit = useCallback/);
  assert.doesNotMatch(hook, /const patch = useCallback/);
  assert.equal((hook.match(/recover\(\(\) => client\./g) ?? []).length, 0,
    'individual setters should use commit instead of duplicating recovery wiring');
});

test('UI translates processing load domain values locally', async () => {
  const types = await read('ui/lib/types.ts');
  const home = await read('ui/features/home/HomeScreen.tsx');
  assert.match(types, /ProcessingLoad/);
  assert.doesNotMatch(types, /loadLabelKey/);
  assert.match(home, /loadLabelKey\(state\.load\)/);
});

test('unavailable virtual output routes are disabled in the UI', async () => {
  const routing = await read('ui/features/routing/RoutingScreen.tsx');
  const segmented = await read('ui/components/SegmentedControl.tsx');
  assert.match(routing, /disabled:\s*!state\.virtualOutputAvailable/);
  assert.match(segmented, /disabled=\{option\.disabled\}/);
});

test('disabled segmented options have an explicit visual state', async () => {
  const css = await read('ui/theme/components.css');
  assert.match(css, /\.segmented button:disabled/);
});

test('native processing switch is disabled until the backend is ready', async () => {
  const shell = await read('ui/app/AppShell.tsx');
  const types = await read('ui/lib/types.ts');
  const home = await read('ui/features/home/HomeScreen.tsx');
  assert.match(types, /ProcessingBackendStatus/);
  assert.match(shell, /backendStatus\s*===\s*'ready'/);
  assert.match(shell, /disabled=\{!processingReady\}/);
  assert.match(home, /backend-notice/);
});
