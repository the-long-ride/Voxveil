import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { VoxveilState } from '../lib/types';

const nativeState: VoxveilState = {
  edition: 'pro-system',
  masterEnabled: true,
  processingMode: 'all',
  engine: 'auto',
  vocalLevel: 12,
  quality: 50,
  outputMode: 'both',
  physicalOutput: 'Native Output',
  virtualOutputAvailable: true,
  estimatedLatencyMs: 42,
  load: 'medium',
  apps: [
    { id: 'browser', name: 'Browser', category: 'media', enabled: true },
    { id: 'call', name: 'Call', category: 'communication', enabled: false, bypassReason: 'communication' },
  ],
};

const client = vi.hoisted(() => ({
  getState: vi.fn(async () => nativeState),
  setMasterEnabled: vi.fn(async () => undefined),
  setProcessingMode: vi.fn(async () => undefined),
  setEngine: vi.fn(async () => undefined),
  setVocalLevel: vi.fn(async () => undefined),
  setQuality: vi.fn(async () => undefined),
  setAppOverride: vi.fn(async () => undefined),
  setOutputRoute: vi.fn(async () => undefined),
}));

vi.mock('../lib/tauri', () => ({ createVoxveilClient: () => client }));

import { useVoxveilState } from './useVoxveilState';

afterEach(() => {
  delete (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__;
  vi.clearAllMocks();
});

describe('useVoxveilState', () => {
  it('hydrates and synchronizes through the Tauri command boundary', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.physicalOutput).toBe('Native Output'));

    act(() => result.current.setMasterEnabled(false));
    act(() => result.current.setProcessingMode('per-app'));
    act(() => result.current.setEngine('dsp'));
    act(() => result.current.setVocalLevel(30));
    act(() => result.current.setQuality(10));
    act(() => result.current.setOutputMode('physical'));
    act(() => result.current.setAppEnabled('browser', false));

    expect(client.setMasterEnabled).toHaveBeenCalledWith(false);
    expect(client.setProcessingMode).toHaveBeenCalledWith('per-app');
    expect(client.setEngine).toHaveBeenCalledWith('dsp');
    expect(client.setVocalLevel).toHaveBeenCalledWith(30);
    expect(client.setQuality).toHaveBeenCalledWith(10);
    expect(client.setOutputRoute).toHaveBeenCalledWith('physical');
    expect(client.setAppOverride).toHaveBeenCalledWith('browser', false);
  });

  it('does not enable communication audio in preview mode', () => {
    const { result } = renderHook(() => useVoxveilState());
    const call = result.current.state.apps.find((app) => app.bypassReason === 'communication');
    if (!call) throw new Error('communication fixture missing');
    act(() => result.current.setAppEnabled(call.id, true));
    expect(result.current.state.apps.find((app) => app.id === call.id)?.enabled).toBe(false);
  });
  it('fails safe when native state hydration is unavailable', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getState.mockRejectedValueOnce(new Error('backend unavailable'));
    const { result } = renderHook(() => useVoxveilState());
    expect(result.current.state.masterEnabled).toBe(false);
    expect(result.current.state.virtualOutputAvailable).toBe(false);
    expect(result.current.state.apps).toEqual([]);
    await waitFor(() => expect(client.getState).toHaveBeenCalledTimes(1));
  });
});
