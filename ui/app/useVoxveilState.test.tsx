import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AudioPlaybackSnapshot, SystemAudioEndpoint, SystemAudioInstallResult, VoxveilState } from '../lib/types';

const stoppedPlayback: AudioPlaybackSnapshot = {
  status: 'stopped',
  fileName: null,
  outputEndpointId: null,
  outputName: null,
  sampleRate: null,
  positionFrames: 0,
  totalFrames: null,
  error: null,
};

const nativeState: VoxveilState = {
  edition: 'pro-system',
  windowsAudioRoutesAvailable: true,
  audioRouteChoice: 'physical-apo',
  audioRouteError: null,
  playback: stoppedPlayback,
  masterEnabled: true,
  backendStatus: 'ready',
  backendDetail: null,
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
  setAudioRoute: vi.fn(async () => nativeState),
  openAudioFile: vi.fn(async () => stoppedPlayback),
  getPlaybackState: vi.fn(async () => stoppedPlayback),
  pausePlayback: vi.fn(async () => undefined),
  resumePlayback: vi.fn(async () => undefined),
  seekPlayback: vi.fn(async () => undefined),
  stopPlayback: vi.fn(async () => undefined),
  listSystemAudioEndpoints: vi.fn(async () => playbackEndpoints),
  listAudioOutputs: vi.fn(async () => []),
  setPhysicalAudioOutput: vi.fn(async () => nativeState),
  installSystemAudioComponent: vi.fn(async (endpointId: string): Promise<SystemAudioInstallResult> => ({ endpointId, outcome: 'launched' })),
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
  cleanup();
  delete (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__;
  client.getState.mockReset().mockResolvedValue(nativeState);
  client.getPlaybackState.mockReset().mockResolvedValue(stoppedPlayback);
  client.openAudioFile.mockReset().mockResolvedValue(stoppedPlayback);
  client.setAudioRoute.mockReset().mockResolvedValue(nativeState);
  client.setPhysicalAudioOutput.mockReset().mockResolvedValue(nativeState);
  client.setMasterEnabled.mockReset().mockResolvedValue(undefined);
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

  it('switches to the owned file route and disables system processing', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.physicalOutput).toBe('Native Output'));
    client.setAudioRoute.mockResolvedValueOnce({
      ...nativeState,
      audioRouteChoice: 'owned-file-playback',
      masterEnabled: false,
      backendStatus: 'unsupported',
      backendKind: null,
    });

    await act(async () => result.current.setAudioRoute('owned-file-playback'));

    expect(client.setAudioRoute).toHaveBeenCalledWith('owned-file-playback');
    expect(result.current.state.audioRouteChoice).toBe('owned-file-playback');
    expect(result.current.state.masterEnabled).toBe(false);
    expect(result.current.canStartProcessing).toBe(false);
  });

  it('does not re-enumerate devices after changing the processing route', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.setAudioRoute.mockResolvedValueOnce({
      ...nativeState,
      audioRouteChoice: 'owned-file-playback',
      masterEnabled: false,
      backendStatus: 'unsupported',
      backendKind: null,
    });
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(client.listSystemAudioEndpoints).toHaveBeenCalledOnce());
    await waitFor(() => expect(client.listAudioOutputs).toHaveBeenCalledOnce());

    await act(async () => result.current.setAudioRoute('owned-file-playback'));

    expect(client.listSystemAudioEndpoints).toHaveBeenCalledOnce();
    expect(client.listAudioOutputs).toHaveBeenCalledOnce();
    expect(client.getState).toHaveBeenCalledOnce();
  });

  it('keeps the prior route and reports a failed route selection', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.setAudioRoute.mockRejectedValueOnce(new Error('route could not start'));
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.audioRouteChoice).toBe('physical-apo'));

    await act(async () => result.current.setAudioRoute('owned-file-playback'));

    expect(result.current.state.audioRouteChoice).toBe('physical-apo');
    expect(result.current.audioRouteError).toBe('route could not start');
  });

  it('opens a local file and routes transport controls through the command boundary', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    const playing: AudioPlaybackSnapshot = {
      ...stoppedPlayback,
      status: 'playing',
      fileName: 'song.flac',
      sampleRate: 48_000,
      totalFrames: 96_000,
      outputEndpointId: 'native-output',
      outputName: 'Native Output',
    };
    client.openAudioFile.mockResolvedValueOnce(playing);
    client.getPlaybackState.mockResolvedValue(playing);
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.audioRouteChoice).toBe('physical-apo'));
    client.setAudioRoute.mockResolvedValueOnce({
      ...nativeState,
      audioRouteChoice: 'owned-file-playback',
      masterEnabled: false,
      backendStatus: 'unsupported',
      backendKind: null,
    });
    await act(async () => result.current.setAudioRoute('owned-file-playback'));
    await act(async () => result.current.openAudioFile());
    await act(async () => result.current.pausePlayback());
    await act(async () => result.current.seekPlayback(48_000));
    await act(async () => result.current.stopPlayback());

    expect(client.openAudioFile).toHaveBeenCalledOnce();
    expect(result.current.state.playback.fileName).toBe('song.flac');
    expect(client.pausePlayback).toHaveBeenCalledOnce();
    expect(client.seekPlayback).toHaveBeenCalledWith(48_000);
    expect(client.stopPlayback).toHaveBeenCalledOnce();
  });

  it('refreshes the owned playback position from the native worker', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getState.mockResolvedValueOnce({
      ...nativeState,
      audioRouteChoice: 'owned-file-playback',
      masterEnabled: false,
      backendStatus: 'unsupported',
      backendKind: null,
    });
    client.getPlaybackState.mockResolvedValue({
      ...stoppedPlayback,
      status: 'playing',
      fileName: 'song.mp3',
      sampleRate: 48_000,
      totalFrames: 96_000,
      positionFrames: 12_345,
    });

    const { result } = renderHook(() => useVoxveilState());

    await waitFor(() => expect(result.current.state.playback.positionFrames).toBe(12_345));
    expect(client.getPlaybackState).toHaveBeenCalled();
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

  it('surfaces reboot-required system audio installs', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.installSystemAudioComponent.mockResolvedValueOnce({
      endpointId: 'speakers',
      outcome: 'reboot-required',
      detail: 'Restart Windows, then run this installation again.',
    });
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.systemAudioEndpoints[0]?.displayName).toBe('Speakers'));

    act(() => result.current.installSystemAudioEndpoint('speakers'));

    await waitFor(() => {
      expect(result.current.systemAudioInstallError).toBe('Restart Windows, then run this installation again.');
    });
  });

  it('allows a configured relay to request processing startup', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getState.mockResolvedValueOnce({
      ...nativeState,
      audioRouteChoice: 'legacy-automatic',
      masterEnabled: false,
      backendStatus: 'routing-required',
      backendKind: 'vb-cable-relay',
      physicalOutputEndpointId: 'speakers',
    });
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.state.backendStatus).toBe('routing-required'));

    expect(result.current.canStartProcessing).toBe(true);
    act(() => result.current.setMasterEnabled(true));

    expect(client.setMasterEnabled).toHaveBeenCalledWith(true);
  });

  it('surfaces a native processing startup error after restoring the backend state', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getState.mockResolvedValue({ ...nativeState, masterEnabled: false });
    client.setMasterEnabled.mockRejectedValueOnce(new Error('Windows audio relay could not start'));
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(result.current.canStartProcessing).toBe(true));

    act(() => result.current.setMasterEnabled(true));

    await waitFor(() => expect(client.setMasterEnabled).toHaveBeenCalledWith(true));
    await waitFor(() => {
      expect(result.current.processingOperationError).toBe('Windows audio relay could not start');
      expect(result.current.state.masterEnabled).toBe(false);
    });
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

  it('refreshes system route status without re-enumerating physical outputs', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(client.listSystemAudioEndpoints).toHaveBeenCalledOnce());
    await waitFor(() => expect(client.listAudioOutputs).toHaveBeenCalledOnce());

    act(() => result.current.refreshSystemAudioEndpoints());

    await waitFor(() => expect(client.listSystemAudioEndpoints).toHaveBeenCalledTimes(2));
    expect(client.getState).toHaveBeenCalledTimes(2);
    expect(client.listAudioOutputs).toHaveBeenCalledOnce();
  });

  it('does not re-enumerate endpoints after selecting a physical output', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    const { result } = renderHook(() => useVoxveilState());
    await waitFor(() => expect(client.listSystemAudioEndpoints).toHaveBeenCalledOnce());
    await waitFor(() => expect(client.listAudioOutputs).toHaveBeenCalledOnce());

    client.setPhysicalAudioOutput.mockResolvedValueOnce({
      ...nativeState,
      physicalOutput: 'Speakers',
      physicalOutputEndpointId: 'speakers',
    });

    await act(async () => result.current.selectPhysicalOutput('speakers'));

    await waitFor(() => expect(client.setPhysicalAudioOutput).toHaveBeenCalledWith('speakers'));
    await waitFor(() => expect(result.current.state.physicalOutputEndpointId).toBe('speakers'));
    expect(client.getState).toHaveBeenCalledOnce();
    expect(client.listSystemAudioEndpoints).toHaveBeenCalledOnce();
    expect(client.listAudioOutputs).toHaveBeenCalledOnce();
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
