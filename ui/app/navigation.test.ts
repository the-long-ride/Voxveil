import { describe, expect, it } from 'vitest';
import { mobileNavigation, NAVIGATION } from './navigation';

describe('navigation', () => {
  it('keeps Engine on desktop but out of mobile bottom navigation', () => {
    expect(NAVIGATION.map((item) => item.id)).toContain('engine');
    expect(mobileNavigation().map((item) => item.id)).not.toContain('engine');
  });
});
