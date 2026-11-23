import { describe, expect, it, vi } from 'vitest';
import { resetRemovedAiEngine } from './modelRemoval';

describe('model removal engine fallback', () => {
  it.each(['dsp', 'auto', 'ai'] as const)('preserves %s unless it explicitly requires the removed AI', (engine) => {
    const setEngine = vi.fn();
    resetRemovedAiEngine(engine, setEngine);
    if (engine === 'ai') expect(setEngine).toHaveBeenCalledWith('auto');
    else expect(setEngine).not.toHaveBeenCalled();
  });
});
