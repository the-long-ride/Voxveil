import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AiModelStatus } from '../../lib/types';

const installed: AiModelStatus = {
  id: 'htdemucs-ft-vocals-fp16',
  displayName: 'HT-Demucs FT Vocals FP16',
  approximateSizeMb: 166,
  license: 'MIT',
  source: 'StemSplitio/htdemucs-ft-vocals-onnx',
  sourceRevision: 'revision',
  installed: true,
  runtimeAvailable: false,
  bundled: false,
  downloadAvailable: true,
  consentRequired: true,
};

const client = vi.hoisted(() => ({
  getAiModelStatus: vi.fn(),
  installAiModel: vi.fn(),
  removeAiModel: vi.fn(),
}));
const listen = vi.hoisted(() => vi.fn(async () => vi.fn()));

vi.mock('../../lib/tauri', () => ({ createVoxveilClient: () => client }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));

import { useAiModelManager } from './useAiModelManager';

afterEach(() => {
  delete (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__;
  vi.clearAllMocks();
});

describe('useAiModelManager', () => {
  it('hydrates status and installs only after the caller supplies consent', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getAiModelStatus.mockResolvedValue({ ...installed, installed: false });
    client.installAiModel.mockResolvedValue(installed);
    const { result } = renderHook(() => useAiModelManager());
    await waitFor(() => expect(client.getAiModelStatus).toHaveBeenCalledTimes(1));

    await act(async () => result.current.install(false));
    expect(client.installAiModel).not.toHaveBeenCalled();

    await act(async () => result.current.install(true));
    expect(client.installAiModel).toHaveBeenCalledWith(installed.id, true);
    expect(result.current.status.installed).toBe(true);
  });

  it('removes the local model and invokes the engine fallback callback', async () => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getAiModelStatus.mockResolvedValue({ ...installed, installed: false });
    client.removeAiModel.mockResolvedValue({ ...installed, installed: false });
    const onRemoved = vi.fn();
    const { result } = renderHook(() => useAiModelManager(onRemoved));
    await waitFor(() => expect(client.getAiModelStatus).toHaveBeenCalledTimes(1));
    await act(async () => result.current.remove());
    expect(client.removeAiModel).toHaveBeenCalled();
    expect(onRemoved).toHaveBeenCalledTimes(1);
  });
});
