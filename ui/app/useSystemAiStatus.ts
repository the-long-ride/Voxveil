import { useEffect, type Dispatch, type SetStateAction } from 'react';
import type { createVoxveilClient } from '../lib/tauri';
import type { VoxveilState } from '../lib/types';

export function useSystemAiStatus(native: boolean, enabled: boolean, client: ReturnType<typeof createVoxveilClient>, setState: Dispatch<SetStateAction<VoxveilState>>) {
  useEffect(() => {
    if (!native || !enabled) return;
    let active = true;
    const refresh = async () => {
      try {
        const systemAiStatus = await client.getSystemAiStatus();
        if (active) setState((current) => ({ ...current, systemAiStatus }));
      } catch { /* Status polling must not interrupt audio controls. */ }
    };
    void refresh();
    const timer = window.setInterval(() => void refresh(), 500);
    return () => { active = false; window.clearInterval(timer); };
  }, [native, enabled, client, setState]);
}
