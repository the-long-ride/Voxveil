import { describe, expect, it, vi } from 'vitest';
import { createVoxveilClient } from './tauri';

describe('tauri client', () => {
  it('maps every UI mutation to a narrow named command', async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const client = createVoxveilClient(invoke);
    await client.setMasterEnabled(false);
    await client.setProcessingMode('per-app');
    await client.setEngine('dsp');
    await client.setVocalLevel(25);
    await client.setQuality(70);
    await client.setAppOverride('browser', true);
    await client.setOutputRoute('both');
    expect(invoke.mock.calls).toEqual([
      ['set_master_enabled', { enabled: false }],
      ['set_processing_mode', { mode: 'per-app' }],
      ['set_engine', { engine: 'dsp' }],
      ['set_vocal_level', { value: 25 }],
      ['set_quality_preference', { value: 70 }],
      ['set_app_override', { id: 'browser', enabled: true }],
      ['set_output_route', { mode: 'both' }],
    ]);
  });

  it('maps state reads to the state command', async () => {
    const state = { masterEnabled: true };
    const invoke = vi.fn().mockResolvedValue(state);
    const client = createVoxveilClient(invoke);
    expect(await client.getState()).toBe(state);
    expect(invoke).toHaveBeenCalledWith('get_app_state');
  });
});
