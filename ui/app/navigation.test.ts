import { describe, expect, it } from 'vitest';
import {
  mobileNavigation,
  navigationForAudioRoute,
  NAVIGATION,
  routeAfterAudioRouteChoiceChanged,
} from './navigation';

describe('navigation', () => {
  it('keeps Engine on desktop but out of mobile bottom navigation', () => {
    expect(NAVIGATION.map((item) => item.id)).toContain('engine');
    expect(mobileNavigation().map((item) => item.id)).not.toContain('engine');
  });

  it('shows Player navigation only for the owned-file route', () => {
    expect(navigationForAudioRoute('physical-apo').map((item) => item.id)).not.toContain('playback');
    expect(navigationForAudioRoute('owned-file-playback').map((item) => item.id)).toContain('playback');
    expect(mobileNavigation('owned-file-playback').map((item) => item.id)).toContain('playback');
    expect(navigationForAudioRoute('owned-file-playback').map((item) => item.id)).toContain('settings');
    expect(mobileNavigation('owned-file-playback').map((item) => item.id)).toContain('settings');
  });

  it('keeps the current page when local-file playback mode is selected', () => {
    expect(routeAfterAudioRouteChoiceChanged('settings', 'owned-file-playback')).toBe('settings');
  });

  it('returns to Home if local-file mode is left while its Player page is open', () => {
    expect(routeAfterAudioRouteChoiceChanged('playback', 'physical-apo')).toBe('home');
  });
});
