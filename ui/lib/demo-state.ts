import type { VoxveilState } from './types';

export const PREVIEW_STATE: VoxveilState = {
  edition: 'standard',
  masterEnabled: true,
  processingMode: 'all',
  engine: 'auto',
  vocalLevel: 18,
  quality: 56,
  outputMode: 'both',
  physicalOutput: 'Built-in Audio',
  virtualOutputAvailable: true,
  estimatedLatencyMs: 64,
  load: 'medium',
  apps: [
    { id: 'spotify', name: 'Spotify', category: 'media', enabled: true },
    { id: 'browser', name: 'Browser', category: 'media', enabled: true },
    { id: 'game', name: 'Game', category: 'game', enabled: false },
    { id: 'discord', name: 'Discord', category: 'communication', enabled: false, bypassReason: 'communication' },
  ],
};

export const SAFE_NATIVE_STATE: VoxveilState = {
  edition: 'standard',
  masterEnabled: false,
  processingMode: 'all',
  engine: 'auto',
  vocalLevel: 100,
  quality: 50,
  outputMode: 'physical',
  physicalOutput: 'System Default',
  virtualOutputAvailable: false,
  estimatedLatencyMs: 0,
  load: 'idle',
  apps: [],
};
