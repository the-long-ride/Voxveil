export type ProcessingMode = 'all' | 'per-app';
export type EngineKind = 'auto' | 'dsp' | 'ai';
export type OutputMode = 'physical' | 'virtual' | 'both';
export type ThemeMode = 'system' | 'light' | 'dark';
export type ProcessingLoad = 'idle' | 'low' | 'medium' | 'high';

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
  processingMode: ProcessingMode;
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
