export type ProcessingMode = 'all' | 'per-app';
export type EngineKind = 'auto' | 'dsp' | 'ai';
export type OutputMode = 'physical' | 'virtual' | 'both';
export type ThemeMode = 'system' | 'light' | 'dark';
export type ProcessingLoad = 'idle' | 'low' | 'medium' | 'high';
export type ProcessingBackendStatus = 'ready' | 'component-required' | 'routing-required' | 'unsupported' | 'faulted';

export interface AppSource {
  id: string;
  name: string;
  category: 'media' | 'game' | 'communication' | 'system';
  enabled: boolean;
  bypassReason?: 'communication';
}

export interface VoxveilState {
  edition: 'standard' | 'pro-system';
  masterEnabled: boolean;
  backendStatus: ProcessingBackendStatus;
  processingMode: ProcessingMode;
  perAppProcessingAvailable: boolean;
  engine: EngineKind;
  vocalLevel: number;
  quality: number;
  outputMode: OutputMode;
  physicalOutput: string;
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
