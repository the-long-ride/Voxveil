import { describe, expect, it } from 'vitest';
import { estimatedLatency } from './latency';

describe('estimatedLatency', () => {
  it('keeps existing estimates for legacy classic profiles', () => {
    expect(estimatedLatency('dsp', 50, 'balanced')).toBe(19);
    expect(estimatedLatency('ai', 50, 'balanced')).toBe(123);
    expect(estimatedLatency('auto', 50, 'balanced')).toBe(60);
  });

  it('reports the separator delay for vocal isolation', () => {
    expect(estimatedLatency('dsp', 50, 'isolation')).toBe(101);
  });
});
