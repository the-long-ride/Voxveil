import { useCallback, useEffect, useMemo, useState } from 'react';
import { PREVIEW_STATE, SAFE_NATIVE_STATE } from '../lib/demo-state';
import { createVoxveilClient } from '../lib/tauri';
import type {
  AudioOutput,
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

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function canRequestProcessingStart(state: VoxveilState): boolean {
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

  useEffect(() => {
    if (!native) return;
    const refresh = () => { void refreshNativeState(); };
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
      await client.installSystemAudioComponent(endpointId);
      await refreshNativeState();
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
      await refreshSystemAudioEndpoints();
    } finally {
      setSystemAudioInstallBusyId(null);
    }
  }, [client, native, refreshNativeState, refreshSystemAudioEndpoints, systemAudioInstallBusyId]);

  const installAllSystemAudioEndpoints = useCallback(async () => {
    if (!native || systemAudioInstallBusyId) return;
    const installable = systemAudioEndpoints.filter((endpoint) => endpoint.status === 'installable');
    const failures: string[] = [];
    for (const endpoint of installable) {
      setSystemAudioInstallBusyId(endpoint.endpointId);
      try {
        await client.installSystemAudioComponent(endpoint.endpointId);
      } catch (error) {
        failures.push(`${endpoint.displayName}: ${errorMessage(error)}`);
      }
    }
    setSystemAudioInstallBusyId(null);
    setSystemAudioInstallError(failures.length ? failures.join('\n') : null);
    await refreshNativeState();
  }, [client, native, refreshNativeState, systemAudioEndpoints, systemAudioInstallBusyId]);

  const selectPhysicalOutput = useCallback(async (endpointId: string) => {
    if (!native) return;
    setSystemAudioInstallError(null);
    try {
      await client.setPhysicalAudioOutput(endpointId);
      await refreshNativeState();
    } catch (error) {
      setSystemAudioInstallError(errorMessage(error));
    }
  }, [client, native, refreshNativeState]);

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

  const setMasterEnabled = (masterEnabled: boolean) => {
    if (masterEnabled && !canStartProcessing) return;
    commit({ masterEnabled }, () => client.setMasterEnabled(masterEnabled));
  };
  const setProcessingMode = (processingMode: ProcessingMode) =>
    commit({ processingMode }, () => client.setProcessingMode(processingMode));
  const setEngine = (engine: EngineKind) =>
    commit(
      (current) => ({ ...current, engine, estimatedLatencyMs: estimatedLatency(engine, current.quality) }),
      () => client.setEngine(engine),
    );
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
    refreshSystemAudioEndpoints: () => { void refreshNativeState(); },
    installSystemAudioEndpoint: (endpointId: string) => { void installSystemAudioEndpoint(endpointId); },
    installAllSystemAudioEndpoints: () => { void installAllSystemAudioEndpoints(); },
    selectPhysicalOutput: (endpointId: string) => { void selectPhysicalOutput(endpointId); },
    openWindowsSoundSettings: () => { void openWindowsSoundSettings(); },
    openVbCableDownload: () => { void openVbCableDownload(); },
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
