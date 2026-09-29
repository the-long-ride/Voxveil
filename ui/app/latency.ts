import type { ClassicSuppressionProfile, EngineKind } from '../lib/types';

const ISOLATION_BASE_MS = 90;

export function estimatedLatency(
  engine: EngineKind,
  quality: number,
  profile: ClassicSuppressionProfile,
): number {
  if (engine === 'dsp') {
    return Math.round((profile === 'isolation' ? ISOLATION_BASE_MS : 8) + quality * 0.22);
  }
  if (engine === 'ai') return Math.round(55 + quality * 1.35);
  return Math.round(24 + quality * 0.72);
}
