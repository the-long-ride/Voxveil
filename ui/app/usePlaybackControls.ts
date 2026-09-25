import { useCallback } from 'react';
import { errorMessage } from '../lib/errors';
import type { createVoxveilClient } from '../lib/tauri';
import type { AudioPlaybackSnapshot } from '../lib/types';

type VoxveilClient = ReturnType<typeof createVoxveilClient>;

export function usePlaybackControls(
  native: boolean,
  client: VoxveilClient,
  refreshPlayback: () => Promise<void>,
  setPlaybackOperationError: (error: string | null) => void,
  onPlaybackLoaded: (playback: AudioPlaybackSnapshot) => void,
) {
  const openAudioFile = useCallback(async () => {
    setPlaybackOperationError(null);
    if (!native) {
      setPlaybackOperationError('Owned file playback is available in the Windows desktop app.');
      return;
    }
    try {
      const playback = await client.openAudioFile();
      onPlaybackLoaded(playback);
    } catch (error) {
      setPlaybackOperationError(errorMessage(error));
      await refreshPlayback();
    }
  }, [client, native, onPlaybackLoaded, refreshPlayback, setPlaybackOperationError]);

  const playbackCommand = useCallback(async (operation: () => Promise<unknown>) => {
    if (!native) return;
    setPlaybackOperationError(null);
    try {
      await operation();
      await refreshPlayback();
    } catch (error) {
      setPlaybackOperationError(errorMessage(error));
      await refreshPlayback();
    }
  }, [native, refreshPlayback, setPlaybackOperationError]);

  const pausePlayback = useCallback(() => playbackCommand(client.pausePlayback), [client, playbackCommand]);
  const resumePlayback = useCallback(() => playbackCommand(client.resumePlayback), [client, playbackCommand]);
  const stopPlayback = useCallback(() => playbackCommand(client.stopPlayback), [client, playbackCommand]);
  const seekPlayback = useCallback((positionFrames: number) =>
    playbackCommand(() => client.seekPlayback(Math.max(0, Math.floor(positionFrames)))),
  [client, playbackCommand]);

  return { openAudioFile, pausePlayback, resumePlayback, seekPlayback, stopPlayback };
}
