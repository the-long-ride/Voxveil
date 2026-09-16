import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { SystemAudioEndpoint, VoxveilState } from '../lib/types';

const nativeState: VoxveilState = {
  edition: 'pro-system',
  masterEnabled: true,
  backendStatus: 'ready',
  backendKind: 'apo',
  processingMode: 'all',
  perAppProcessingAvailable: false,
  engine: 'auto',
  classicSuppressionProfile: 'music-preservation',
  vocalLevel: 12,
  quality: 50,
  outputMode: 'both',
  physicalOutput: 'Native Output',
  physicalOutputEndpointId: 'native-output',
  virtualOutputAvailable: true,
  estimatedLatencyMs: 42,
  load: 'medium',
  apps: [
    { id: 'browser', name: 'Browser', category: 'media', enabled: true },
    { id: 'call', name: 'Call', category: 'communication', enabled: false, bypassReason: 'communication' },
  ],
};

const playbackEndpoints: SystemAudioEndpoint[] = [
  {
    endpointId: 'speakers',
    displayName: 'Speakers',
    adapterName: 'Example Audio',
    isDefault: true,
    status: 'installable',
  },
];

const client = vi.hoisted(() => ({
  getState: vi.fn(async () => nativeState),
  listSystemAudioEndpoints: vi.fn(async () => playbackEndpoints),
  installSystemAudioComponent: vi.fn(async (endpointId: string) => ({ endpointId, outcome: 'launched' as const })),
  setMasterEnabled: vi.fn(async () => undefined),
  setProcessingMode: vi.fn(async () => undefined),
  setEngine: vi.fn(async () => undefined),
  setClassicSuppressionProfile: vi.fn(async () => undefined),
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
    act(() => result.current.setClassicSuppressionProfile('balanced'));
    act(() => result.current.setVocalLevel(30));
    act(() => result.current.setQuality(10));
    act(() => result.current.setOutputMode('physical'));
    act(() => result.current.setAppEnabled('browser', false));

    expect(client.setMasterEnabled).toHaveBeenCalledWith(false);
    expect(client.setProcessingMode).toHaveBeenCalledWith('per-app');
    expect(client.setEngine).toHaveBeenCalledWith('dsp');
    expect(client.setClassicSuppressionProfile).toHaveBeenCalledWith('balanced');
    expect(client.setVocalLevel).toHaveBeenCalledWith(30);
    expect(client.setQuality).toHaveBeenCalledWith(10);
    expect(client.setOutputRoute).toHaveBeenCalledWith('physical');
    expect(client.setAppOverride).toHaveBeenCalledWith('browser', false);
  });

  it('rolls back the classic profile when the native backend rejects it', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.setClassicSuppressionProfile.mockRejectedValueOnce(new Error('profile update failed'));
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.classicSuppressionProfile).toBe('music-preservation'));

    act(() => result.current.setClassicSuppressionProfile('balanced'));

    expect(client.setClassicSuppressionProfile).toHaveBeenCalledWith('balanced');
    await waitFor(() => expect(result.current.state.classicSuppressionProfile).toBe('music-preservation'));
  });

  it('rolls back the vocal level when the native backend rejects it', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.setVocalLevel.mockRejectedValueOnce(new Error('vocal update failed'));
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.vocalLevel).toBe(12));

    act(() => result.current.setVocalLevel(30));

    expect(client.setVocalLevel).toHaveBeenCalledWith(30);
    await waitFor(() => expect(result.current.state.vocalLevel).toBe(12));
  });

  it('hydrates discovered playback endpoints and installs by opaque endpoint id', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.systemAudioEndpoints[0]?.displayName).toBe('Speakers'));

    act(() => result.current.installSystemAudioEndpoint('speakers'));

    await waitFor(() => expect(client.installSystemAudioComponent).toHaveBeenCalledWith('speakers'));
  });

  it('allows a configured relay to request processing startup', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getState.mockResolvedValueOnce({
      ...nativeState,
      masterEnabled: false,
      backendStatus: 'routing-required',
      backendKind: 'vb-cable-relay',
      physicalOutputEndpointId: 'speakers',
    });
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.backendStatus).toBe('routing-required'));

    expect(result.current.canStartProcessing).toBe(true;
    act(() => result.current.setMasterEnabled(true));

    expect(client.setMasterEnabled).toHaveBeenCalledWith(true);
  });

  it('does not optimistically enable processing when the native backend is unavailable', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getState.mockResolvedValueOnce({ ...nativeState, masterEnabled: false, backendStatus: 'component-required' });
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.backendStatus).toBe('component-required'));

    act(() => result.current.setMasterEnabled(true));

    expect(result.current.state.masterEnabled).toBe(false);
    expect(client.setMasterEnabled).not.toHaveBeenCalled();
  });

  it('refreshes backend readiness when the native window regains focus', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getState
      .mockResolvedValueOnce({ ...nativeState, masterEnabled: false, backendStatus: 'routing-required' })
      .mockResolvedValueOnce({ ...nativeState, masterEnabled: false, backendStatus: 'ready' });
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.backendStatus).toBe('routing-required'));

    act(() => window.dispatchEvent(new Event('focus')));

    await waitFor(() => expect(result.current.state.backendStatus).toBe('ready'));
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
