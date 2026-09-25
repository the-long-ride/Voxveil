export type ProcessingMode = 'all' | 'per-app';
export type EngineKind = 'auto' | 'dsp' | 'ai';
export type AudioRouteChoice = 'legacy-automatic' | 'physical-apo' | 'owned-file-playback';
export type AudioPlaybackStatus = 'stopped' | 'loading' | 'playing' | 'paused' | 'ended' | 'faulted';
export type ClassicSuppressionProfile = 'music-preservation' | 'balanced' | 'strong';
export type OutputMode = 'physical' | 'virtual' | 'both';
export type ThemeMode = 'system' | 'light' | 'dark';
export type ProcessingLoad = 'idle' | 'low' | 'medium' | 'high';
export type ProcessingBackendStatus = 'ready' | 'component-required' | 'routing-required' | 'unsupported' | 'faulted';
export type WindowsInterceptionKind = 'apo' | 'vb-cable-relay' | 'voxveil-cable-relay';
export type SystemAudioEndpointStatus = 'ready' | 'installable' | 'component-required' | 'ambiguous' | 'unsupported';

export interface AppSource {
  id: string;
  name: string;
  category: 'media' | 'game' | 'communication' | 'system';
  enabled: boolean;
  bypassReason?: 'communication';
}

export interface AudioOutput {
  endpointId: string;
  displayName: string;
  isDefault: boolean;
}

export interface AudioPlaybackSnapshot {
  status: AudioPlaybackStatus;
  fileName: string | null;
  outputEndpointId: string | null;
  outputName: string | null;
  sampleRate: number | null;
  positionFrames: number;
  totalFrames: number | null;
  error: string | null;
}

export interface SystemAudioEndpoint {
  endpointId: string;
  displayName: string;
  adapterName?: string;
  isDefault: boolean;
  status: SystemAudioEndpointStatus;
  detail?: string;
}

export interface SystemAudioInstallResult {
  endpointId: string;
  outcome: 'launched' | 'reboot-required' | 'cancelled' | 'device-changed' | 'installed-not-loaded';
  detail?: string;
}

export interface VoxveilState {
  edition: 'standard' | 'pro-system';
  windowsAudioRoutesAvailable: boolean;
  audioRouteChoice: AudioRouteChoice;
  audioRouteError: string | null;
  playback: AudioPlaybackSnapshot;
  masterEnabled: boolean;
  backendStatus: ProcessingBackendStatus;
  backendDetail: string | null;
  backendKind: WindowsInterceptionKind | null;
  processingMode: ProcessingMode;
  perAppProcessingAvailable: boolean;
  engine: EngineKind;
  classicSuppressionProfile: ClassicSuppressionProfile;
  vocalLevel: number;
  quality: number;
  outputMode: OutputMode;
  physicalOutput: string;
  physicalOutputEndpointId: string | null;
  virtualOutputAvailable: boolean;
  estimatedLatencyMs: number;
  load: ProcessingLoad;
  apps: AppSource[];
}

export interface AiModelStatus {
  id: string;
  displayName: string;
  approximateSizeMb: number;
  license: string;
  source: string;
  sourceRevision: string;
  installed: boolean;
  runtimeAvailable: boolean;
  bundled: boolean;
  downloadAvailable: boolean;
  consentRequired: boolean;
}

export interface AiModelDownloadProgress {
  modelId: string;
  downloadedBytes: number;
  totalBytes: number | null;
}
