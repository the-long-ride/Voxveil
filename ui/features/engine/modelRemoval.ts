import type { EngineKind } from '../../lib/types';

export function resetRemovedAiEngine(engine: EngineKind, setEngine: (next: EngineKind) => void) {
  if (engine === 'ai') setEngine('auto');
}
