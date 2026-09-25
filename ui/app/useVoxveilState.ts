import { useCallback, useEffect, useMemo, useState } from 'react';
import { PREVIEW_STATE, SAFE_NATIVE_STATE } from '../lib/demo-state';
import { errorMessage } from '../lib/errors';
import { createVoxveilClient } from '../lib/tauri';
import { usePlaybackControls } from './usePlaybackControls';
import type {
  AudioOutput,
  AudioPlaybackSnapshot,
  AudioRouteChoice,
  ClassicSuppressionProfile,
  EngineKind,
  OutputMode,
  ProcessingMode,
  SystemAudioEndpoint,
  VoxveilState,
} from '../lib/types';

type StateUpdate = Partial<VoxveilState> | ((current: VoxveilState) => VoxveilState);

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function estimatedLatency(engine: EngineKind, quality: number): number {
  if (engine === 'dsp') return Math.round(8 + quality * 0.22);
  if (engine === 'ai') return Math.round(55 + quality * 1.35);
  return Math.round(24 + quality * 0.72);
}

function canRequestProcessingStart(state: VoxveilState): boolean {
  if (state.audioRouteChoice === 'owned-file-playback') return false;
  if (state.backendStatus === 'ready') return true;
  if (state.backendStatus !== 'routing-required' || state.physicalOutputEndpointId === null) {
    return false;
  }
  return state.backendKind === 'vb-cable-relay' || state.backendKind === 'voxveil-cable-relay';
}

export function useVoxveilState() {
  const native = isTauriRuntime();
  const [state, setState] = useState<VoxveilState>(() => native ? SAFE_NATIVE_STATE : PREVIEW_STATE);
  const [systemAudioEndpoints, setSystemAudioEndpoints] = useState<SystemAudioEndpoint[]>([]);
  const [physicalOutputs, setPhysicalOutputs] = useState<AudioOutput[]>([]);
  const [systemAudioEndpointsBusy, setSystemAudioEndpointsBusy] = useState(false);
  const [systemAudioInstallBusyId, setSystemAudioInstallBusyId] = useState<string | null>(null);
  const [systemAudioInstallError, setSystemAudioInstallError] = useState<string | null>(null);
  const [audioRouteError, setAudioRouteError] = useState<string | null>(null);
  const [processingOperationError, setProcessingOperationError] = useState<string | null>(null);
  const [playbackOperationError, setPlaybackOperationError] = useState<string | null>(null);
  const client = useMemo(() => createVoxveilClient(), []);
  const canStartProcessing = canRequestProcessingStart(state);

  const refreshSystemAudioEndpoints = useCallback(async () => {
    if (!native) return;
    setSystemAudioEndpointsBusy(true);
    try {
      setSystemAudioEndpoints(await client.listSystemAudioEndpoints());
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
    } finally {
      setSystemAudioEndpointsBusy(false);
    }
  }, [client, native]);
  const refreshAudioRouteStatus = useCallback(async () => {
    if (!native) return;
    setSystemAudioEndpointsBusy(true);
    try {
      const [nextState, endpoints] = await Promise.all([client.getState(), client.listSystemAudioEndpoints()]);
      setState(nextState);
      setSystemAudioEndpoints(endpoints);
      setSystemAudioInstallError(null);
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
    } finally {
      setSystemAudioEndpointsBusy(false);
    }
  }, [client, native]);

  const refreshPhysicalOutputs = useCallback(async () => {
    if (!native) return;
    try {
      setPhysicalOutputs(await client.listAudioOutputs());
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
    }
  }, [client, native]);

  const refreshNativeState = useCallback(async () => {
    if (!native) return;
    try { setState(await client.getState()); } catch { setState(SAFE_NATIVE_STATE); }
    await Promise.all([refreshSystemAudioEndpoints(), refreshPhysicalOutputs()]);
  }, [client, native, refreshPhysicalOutputs, refreshSystemAudioEndpoints]);

  const refreshPlayback = useCallback(async () => {
    if (!native || state.audioRouteChoice !== 'owned-file-playback') return;
    try {
      const playback = await client.getPlaybackState();
      setState((current) => ({ ...current, playback }));
    } catch (error) {
      setPlaybackOperationError(errorMessage(error));
    }
  }, [client, native, state.audioRouteChoice]);

  const updatePlayback = useCallback((playback: AudioPlaybackSnapshot) => {
    setState((current) => ({ ...current, playback }));
  }, []);

  useEffect(() => {
    if (!native || state.audioRouteChoice !== 'owned-file-playback') return;
    let active = true;
    const refresh = async () => {
      if (!active) return;
      try {
        const playback = await client.getPlaybackState();
        if (active) setState((current) => ({ ...current, playback }));
      } catch (error) {
        if (active) setPlaybackOperationError(errorMessage(error));
      }
    };
    void refresh();
    const timer = window.setInterval(() => { void refresh(); }, 400);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [client, native, state.audioRouteChoice]);

  useEffect(() => {
    if (!native) return;
    const refresh = () => void refreshNativeState();
    refresh();
    window.addEventListener('focus', refresh);
    return () => window.removeEventListener('focus', refresh);
  }, [native, refreshNativeState]);

  const recover = useCallback((operation: () => Promise<unknown>) => {
    if (!native) return;
    void operation().catch(async () => {
      try { setState(await client.getState()); } catch { setState(SAFE_NATIVE_STATE); }
    });
  }, [client, native]);

  const commit = useCallback((next: StateUpdate, operation: () => Promise<unknown>) => {
    setState((current) => typeof next === 'function' ? next(current) : { ...current, ...next });
    recover(operation);
  }, [recover]);

  const installSystemAudioEndpoint = useCallback(async (endpointId: string) => {
    if (!native || systemAudioInstallBusyId) return;
    setSystemAudioInstallBusyId(endpointId);
    setSystemAudioInstallError(null);
    try {
      const result = await client.installSystemAudioComponent(endpointId);
      if (result.outcome === 'reboot-required') {
        setSystemAudioInstallError(result.detail ?? 'Windows must restart before the Voxveil system-audio installation can continue.');
        await refreshSystemAudioEndpoints();
        return;
      }
      await refreshNativeState();
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
      await refreshSystemAudioEndpoints();
    } finally {
      setSystemAudioInstallBusyId(null);
    }
  }, [client, native, refreshNativeState, refreshSystemAudioEndpoints, systemAudioInstallBusyId]);

  const selectPhysicalOutput = useCallback(async (endpointId: string) => {
    if (!native) return;
    setSystemAudioInstallError(null);
    try {
      setState(await client.setPhysicalAudioOutput(endpointId));
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
      try { setState(await client.getState()); } catch { setState(SAFE_NATIVE_STATE); }
    }
  }, [client, native]);

  const openWindowsSoundSettings = useCallback(async () => {
    if (!native) return;
    try {
      await client.openWindowsSoundSettings();
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
    }
  }, [client, native]);

  const openVbCableDownload = useCallback(async () => {
    if (!native) return;
    try {
      await client.openVbCableDownload();
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
    }
  }, [client, native]);

  const setAudioRoute = useCallback(async (audioRouteChoice: AudioRouteChoice) => {
    setAudioRouteError(null);
    if (!native) {
      setState((current) => ({
        ...current,
        audioRouteChoice,
        masterEnabled: audioRouteChoice === 'owned-file-playback' ? false : current.masterEnabled,
        audioRouteError: null,
      }));
      return;
    }
    try {
      setState(await client.setAudioRoute(audioRouteChoice));
    } catch (error) {
      setAudioRouteError(errorMessage(error));
      try { setState(await client.getState()); } catch { setState(SAFE_NATIVE_STATE); }
    }
  }, [client, native]);

  const playbackControls = usePlaybackControls(
    native,
    client,
    refreshPlayback,
    setPlaybackOperationError,
    updatePlayback,
  );

  const setMasterEnabled = (masterEnabled: boolean) => {
    if (masterEnabled && !canStartProcessing) return;
    setProcessingOperationError(null);
    commit({ masterEnabled }, async () => {
      try {
        await client.setMasterEnabled(masterEnabled);
      } catch (error) {
        setProcessingOperationError(errorMessage(error));
        throw error;
      }
    });
  };
  const setProcessingMode = (processingMode: ProcessingMode) =>
    commit({ processingMode }, () => client.setProcessingMode(processingMode));
  const setEngine = (engine: EngineKind) => {
    if (state.audioRouteChoice === 'owned-file-playback' && engine !== 'dsp') return;
    commit(
      (current) => ({ ...current, engine, estimatedLatencyMs: estimatedLatency(engine, current.quality) }),
      () => client.setEngine(engine),
    );
  };
  const setClassicSuppressionProfile = (classicSuppressionProfile: ClassicSuppressionProfile) =>
    commit(
      { classicSuppressionProfile },
      () => client.setClassicSuppressionProfile(classicSuppressionProfile),
    );
  const setVocalLevel = (vocalLevel: number) =>
    commit({ vocalLevel }, () => client.setVocalLevel(vocalLevel));
  const setQuality = (quality: number) =>
    commit(
      (current) => ({ ...current, quality, estimatedLatencyMs: estimatedLatency(current.engine, quality) }),
      () => client.setQuality(quality),
    );
  const setOutputMode = (outputMode: OutputMode) =>
    commit({ outputMode }, () => client.setOutputRoute(outputMode));
  const setAppEnabled = (id: string, enabled: boolean) =>
    commit(
      (current) => ({
        ...current,
        apps: current.apps.map((app) => {
          if (app.id !== id || (enabled && app.bypassReason === 'communication')) return app;
          return { ...app, enabled };
        }),
      }),
      () => client.setAppOverride(id, enabled),
    );

  return {
    state,
    canStartProcessing,
    systemAudioEndpoints,
    physicalOutputs,
    systemAudioEndpointsBusy,
    systemAudioInstallBusyId,
    systemAudioInstallError,
    audioRouteError,
    processingOperationError,
    playbackOperationError,
    refreshSystemAudioEndpoints: () => { void refreshAudioRouteStatus(); },
    installSystemAudioEndpoint: (endpointId: string) => { void installSystemAudioEndpoint(endpointId); },
    selectPhysicalOutput,
    openWindowsSoundSettings: () => { void openWindowsSoundSettings(); },
    openVbCableDownload: () => { void openVbCableDownload(); },
    setAudioRoute,
    ...playbackControls,
    setMasterEnabled,
    setProcessingMode,
    setEngine,
    setClassicSuppressionProfile,
    setVocalLevel,
    setQuality,
    setOutputMode,
    setAppEnabled,
  };
}

export type VoxveilModel = ReturnType<typeof useVoxveilState>;
