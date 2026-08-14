import { useCallback, useEffect, useMemo, useState } from 'react';
import { PREVIEW_STATE, SAFE_NATIVE_STATE } from '../lib/demo-state';
import { createVoxveilClient } from '../lib/tauri';
import type { EngineKind, OutputMode, ProcessingMode, VoxveilState } from '../lib/types';

type StateUpdate = Partial<VoxveilState> | ((current: VoxveilState) => VoxveilState);

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function estimatedLatency(engine: EngineKind, quality: number): number {
  if (engine === 'dsp') return Math.round(8 + quality * 0.22);
  if (engine === 'ai') return Math.round(55 + quality * 1.35);
  return Math.round(24 + quality * 0.72);
}

export function useVoxveilState() {
  const native = isTauriRuntime();
  const [state, setState] = useState<VoxveilState>(() => native ? SAFE_NATIVE_STATE : PREVIEW_STATE);
  const client = useMemo(() => createVoxveilClient(), []);

  useEffect(() => {
    if (!native) return;
    void client.getState().then(setState).catch(() => undefined);
  }, [client, native]);

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

  const setMasterEnabled = (masterEnabled: boolean) =>
    commit({ masterEnabled }, () => client.setMasterEnabled(masterEnabled));
  const setProcessingMode = (processingMode: ProcessingMode) =>
    commit({ processingMode }, () => client.setProcessingMode(processingMode));
  const setEngine = (engine: EngineKind) =>
    commit(
      (current) => ({ ...current, engine, estimatedLatencyMs: estimatedLatency(engine, current.quality) }),
      () => client.setEngine(engine),
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
    setMasterEnabled,
    setProcessingMode,
    setEngine,
    setVocalLevel,
    setQuality,
    setOutputMode,
    setAppEnabled,
  };
}

export type VoxveilModel = ReturnType<typeof useVoxveilState>;
