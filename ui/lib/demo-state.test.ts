import { describe, expect, it } from 'vitest';
import { SAFE_NATIVE_STATE } from './demo-state';

describe('native initial state', () => {
  it('uses the automatic route without claiming processing is active', () => {
    expect(SAFE_NATIVE_STATE.audioRouteChoice).toBe('legacy-automatic');
    expect(SAFE_NATIVE_STATE.masterEnabled).toBe(false);
  });
});
