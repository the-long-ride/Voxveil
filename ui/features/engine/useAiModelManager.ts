import { useCallback, useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { createVoxveilClient } from '../../lib/tauri';
import type { AiModelDownloadProgress, AiModelStatus } from '../../lib/types';

const PREVIEW_STATUS: AiModelStatus = {
  id: 'htdemucs-ft-vocals-fp16',
  displayName: 'HT-Demucs FT Vocals FP16',
  approximateSizeMb: 166,
  license: 'MIT',
  source: 'StemSplitio/htdemucs-ft-vocals-onnx',
  sourceRevision: '2ef0d757d3e226d0da85fb8c71514f464fcabdd0',
  installed: false,
  runtimeAvailable: false,
  bundled: false,
  downloadAvailable: false,
  consentRequired: true,
};

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function useAiModelManager(onRemoved?: () => void) {
  const native = isTauriRuntime();
  const client = useMemo(() => createVoxveilClient(), []);
  const [status, setStatus] = useState<AiModelStatus>(PREVIEW_STATUS);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<AiModelDownloadProgress | null>(null);
  const [error, setError] = useState<'status' | 'download' | 'remove' | null>(null);

  useEffect(() => {
    if (!native) return;
    void client.getAiModelStatus()
      .then((next) => { setStatus(next); setError(null); })
      .catch(() => setError('status'));
  }, [client, native]);

  useEffect(() => {
    if (!native) return;
    let dispose: (() => void) | undefined;
    let disposed = false;
    void listen<AiModelDownloadProgress>('ai-model-download-progress', (event) => setProgress(event.payload))
      .then((unlisten) => { if (disposed) unlisten(); else dispose = unlisten; });
    return () => { disposed = true; dispose?.(); };
  }, [native]);

  const install = useCallback(async (acceptedTerms: boolean) => {
    if (!native || !acceptedTerms) return;
    setBusy(true); setError(null); setProgress(null);
    try { setStatus(await client.installAiModel(status.id, acceptedTerms)); }
    catch { setError('download'); }
    finally { setBusy(false); }
  }, [client, native, status.id]);

  const remove = useCallback(async () => {
    if (!native) return;
    setBusy(true); setError(null); setProgress(null);
    try {
      setStatus(await client.removeAiModel(status.id));
      onRemoved?.();
    } catch { setError('remove'); }
    finally { setBusy(false); }
  }, [client, native, onRemoved, status.id]);

  return { status, native, busy, progress, error, install, remove };
}

export type AiModelController = ReturnType<typeof useAiModelManager>;
