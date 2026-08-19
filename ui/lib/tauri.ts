import { invoke } from '@tauri-apps/api/core';
import type { AiModelStatus, EngineKind, OutputMode, ProcessingMode, VoxveilState } from './types';

type InvokeFn = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createVoxveilClient(call: InvokeFn = invoke) {
  return {
    getState: () => call<VoxveilState>('get_app_state'),
    installSystemAudioComponent: () => call<void>('install_system_audio_component'),
    setMasterEnabled: (enabled: boolean) => call<void>('set_master_enabled', { enabled }),
    setProcessingMode: (mode: ProcessingMode) => call<void>('set_processing_mode', { mode }),
    setEngine: (engine: EngineKind) => call<void>('set_engine', { engine }),
    setVocalLevel: (value: number) => call<void>('set_vocal_level', { value }),
    setQuality: (value: number) => call<void>('set_quality_preference', { value }),
    setAppOverride: (id: string, enabled: boolean) => call<void>('set_app_override', { id, enabled }),
    setOutputRoute: (mode: OutputMode) => call<void>('set_output_route', { mode }),
    getAiModelStatus: () => call<AiModelStatus>('get_ai_model_status'),
    installAiModel: (modelId: string, acceptedTerms: boolean) => call<AiModelStatus>('install_ai_model', { modelId, acceptedTerms }),
    removeAiModel: (modelId: string) => call<AiModelStatus>('remove_ai_model', { modelId }),
  };
}
