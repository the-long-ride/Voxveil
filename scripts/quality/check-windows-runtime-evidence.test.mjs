import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const script = readFileSync('scripts/evaluation/collect-windows-runtime-evidence.ps1', 'utf8');

test('Windows runtime evidence collector captures release-relevant machine state into the ignored evaluation workspace', () => {
  assert.match(script, /\.local-evaluation\\classic-dsp/i);
  assert.match(script, /Get-CimInstance\s+Win32_OperatingSystem/i);
  assert.match(script, /Get-CimInstance\s+Win32_Processor/i);
  assert.match(script, /NumberOfLogicalProcessors/i);
  assert.match(script, /Confirm-SecureBootUEFI/i);
  assert.match(script, /\[Environment\]::SystemDirectory[\s\S]*bcdedit\.exe/i);
  assert.match(script, /testsigning/i);
});

test('Windows runtime evidence collector keeps CPU sampling explicit and manual latency/dropout fields pending', () => {
  assert.match(script, /TotalProcessorTime\.TotalSeconds/i);
  assert.match(script, /CpuSampleSeconds/i);
  assert.match(script, /normalizedPercent/i);
  assert.match(script, /startCpuById/i);
  assert.match(script, /endCpuById/i);
  assert.match(script, /matchedProcessCount/i);
  assert.match(script, /process-changed/i);
  assert.match(script, /dropoutCount\s*=\s*\$null/i);
  assert.match(script, /endToEndLatencyMs\s*=\s*\$null/i);
  assert.match(script, /status\s*=\s*'pending'/i);
  assert.match(script, /remain manual measurements/i);
  assert.doesNotMatch(script, /SerialNumber|MachineName|UserName/i);
});
