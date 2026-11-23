import { useCallback, useEffect, useId, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { createVoxveilClient } from '../../lib/tauri';
import type { AiModelDownloadProgress, AiModelId, AiModelStatus } from '../../lib/types';

const PREVIEW_STATUS: AiModelStatus = {
  id: 'spleeter-2stems-fp16',
  displayName: 'Spleeter 2-stems FP16',
  approximateSizeMb: 40,
  license: 'MIT',
  source: 'csukuangfj/sherpa-onnx-spleeter-2stems-fp16',
  sourceRevision: '93ba771920ade509f8cbd6825b1a90856c797e08',
  attribution: 'Spleeter by Deezer Research; ONNX conversion by sherpa-onnx/k2-fsa.',
  installed: false,
  runtimeAvailable: false,
  bundled: false,
  downloadAvailable: false,
  consentRequired: true,
};

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function preview(modelId?: AiModelId): AiModelStatus {
  return modelId === 'mdx-inst-hq3' ? { ...PREVIEW_STATUS, id: modelId, displayName: 'UVR MDX Inst HQ3', approximateSizeMb: 67, source: 'Anjok/model-repo', sourceRevision: '048ddcf9a0db28453d71943c1e18a1e25291a642', attribution: 'Ultimate Vocal Remover (UVR), Anjok07 and aufr33; MDX-Net by KUIELab / Woosung Choi.' } : PREVIEW_STATUS;
}

export function useAiModelManager(onRemoved?: () => void, modelId?: AiModelId) {
  const listenerId = useId();
  const native = isTauriRuntime();
  const client = useMemo(() => createVoxveilClient(), []);
  const [status, setStatus] = useState<AiModelStatus>(() => preview(modelId));
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<AiModelDownloadProgress | null>(null);
  const [error, setError] = useState<'status' | 'download' | 'remove' | null>(null);

  useEffect(() => {
    let active = true;
    setStatus(preview(modelId));
    const refresh = (event?: Event) => {
      if (event instanceof CustomEvent && event.detail === listenerId) return;
      if (!native) return;
      void client.getAiModelStatus(modelId)
        .then((next) => { if (active) { setStatus(next); setError(null); } })
        .catch(() => { if (active) setError('status'); });
    };
    refresh();
    window.addEventListener('ai-models-changed', refresh);
    return () => { active = false; window.removeEventListener('ai-models-changed', refresh); };
  }, [client, native, modelId, listenerId]);

  useEffect(() => {
    if (!native) return;
    let dispose: (() => void) | undefined;
    let disposed = false;
    void listen<AiModelDownloadProgress>('ai-model-download-progress', (event) => { if (event.payload.modelId === status.id) setProgress(event.payload); })
      .then((unlisten) => { if (disposed) unlisten(); else dispose = unlisten; });
    return () => { disposed = true; dispose?.(); };
  }, [native, status.id]);

  const install = useCallback(async (acceptedTerms: boolean) => {
    if (!native || !acceptedTerms) return;
    setBusy(true); setError(null); setProgress(null);
    try { setStatus(await client.installAiModel(status.id, acceptedTerms)); window.dispatchEvent(new CustomEvent('ai-models-changed', { detail: listenerId })); }
    catch { setError('download'); }
    finally { setBusy(false); }
  }, [client, native, status.id, listenerId]);

  const remove = useCallback(async () => {
    if (!native) return;
    setBusy(true); setError(null); setProgress(null);
    try {
      setStatus(await client.removeAiModel(status.id));
      window.dispatchEvent(new CustomEvent('ai-models-changed', { detail: listenerId }));
      onRemoved?.();
    } catch { setError('remove'); }
    finally { setBusy(false); }
  }, [client, native, onRemoved, status.id, listenerId]);

  return { status, native, busy, progress, error, install, remove };
}

export type AiModelController = ReturnType<typeof useAiModelManager>;
