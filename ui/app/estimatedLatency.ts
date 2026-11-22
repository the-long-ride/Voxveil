import type { EngineKind } from '../lib/types';

export function estimatedLatency(engine: EngineKind, quality: number): number {
  if (engine === 'dsp') return Math.round(8 + quality * 0.22);
  if (engine === 'ai') return Math.round(55 + quality * 1.35);
  return Math.round(24 + quality * 0.72);
}
