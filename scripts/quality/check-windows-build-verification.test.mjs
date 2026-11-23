import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

test('manual Windows build runs the full repository verification gate', async () => {
  const packageJson = JSON.parse(await readFile('package.json', 'utf8'));
  const command = packageJson.scripts?.['build:windows'] ?? '';

  for (const expected of [
    'npm ci --ignore-scripts --no-fund --no-audit',
    'cargo test --workspace',
    'npm test',
    'npm run typecheck',
    'npm run quality',
    'npm run build:windows-driver:x64',
    '-SkipNpmInstall',
  ]) {
    assert.match(command, new RegExp(expected.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }
  assert.doesNotMatch(command, /-SkipTests\b/);
});

test('manual Windows build defaults to driverless and gates the full build on explicit opt-in', async () => {
  const workflow = (await readFile('.github/workflows/manual-build.yml', 'utf8')).replace(/\r\n/g, '\n');
  assert.match(workflow, /name: Build Windows package\n\s+if: \$\{\{ !inputs\.windows_drivers \}\}\n\s+shell: pwsh\n\s+run: npm run build:windows:driverless\n/);
  assert.match(workflow, /name: Build Windows package with drivers\n\s+if: \$\{\{ inputs\.windows_drivers \}\}\n\s+shell: pwsh\n\s+run: npm run build:windows\n/);
  assert.equal((workflow.match(/run: npm run build:windows(?:\s|:)/g) ?? []).length, 2);
  const upload = workflow.split('- name: Upload Windows package')[1].split('- name:')[0];
  assert.doesNotMatch(upload, /\bif:/);
  assert.match(upload, /path: \$\{\{ inputs\.windows_drivers && 'dist\/windows-x64\/Voxveil\/' \|\| 'dist\/windows-x64\/Voxveil-Driverless\/' \}\}/);
});

test('opt-in Windows driver build preserves the validated unsigned driver submission separately', async () => {
  const workflow = (await readFile('.github/workflows/manual-build.yml', 'utf8')).replace(/\r\n/g, '\n');
  assert.match(workflow, /name:\s*Voxveil-windows-driver-submission-\$\{\{\s*github\.sha\s*\}\}/);
  assert.match(workflow, /path:\s*native\/windows\/driver\/out\/x64\/submission\/?/);
  assert.match(workflow, /uses:\s*actions\/upload-artifact@v6/);
  assert.match(workflow, /name: Upload unsigned driver submission\n\s+if: \$\{\{ inputs\.windows_drivers \}\}/);
});

test('manual Windows packages cache catalog-pinned models and stage them for both variants', async () => {
  const workflow = await readFile('.github/workflows/manual-build.yml', 'utf8');
  assert.match(workflow, /uses: actions\/cache@v5/);
  assert.match(workflow, /path: target\/ai-model-cache/);
  assert.match(workflow, /key: windows-ai-models-\$\{\{ hashFiles\('tauri\/models\/catalog\.json'\) \}\}/);
  const stage = workflow.split('- name: Stage pinned AI models')[1].split('- name:')[0];
  assert.doesNotMatch(stage, /\bif:/);
  assert.match(stage, /node scripts\/windows\/stage-ai-models\.mjs/);
  assert.ok(workflow.indexOf('- name: Stage pinned AI models') < workflow.indexOf('- name: Build Windows package'));
});
