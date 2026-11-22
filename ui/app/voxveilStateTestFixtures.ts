import type { AudioPlaybackSnapshot, SystemAudioEndpoint, VoxveilState } from '../lib/types';

export const stoppedPlayback: AudioPlaybackSnapshot = {
  status: 'stopped',
  fileName: null,
  outputEndpointId: null,
  outputName: null,
  sampleRate: null,
  positionFrames: 0,
  totalFrames: null,
  error: null,
};

export const nativeState: VoxveilState = {
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

export const playbackEndpoints: SystemAudioEndpoint[] = [
  {
    endpointId: 'speakers',
    displayName: 'Speakers',
    adapterName: 'Example Audio',
    isDefault: true,
    status: 'installable',
  },
];
