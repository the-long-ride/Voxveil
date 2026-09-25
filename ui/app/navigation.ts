import type { AudioRouteChoice } from '../lib/types';

export type RouteId = 'home' | 'apps' | 'routing' | 'engine' | 'settings' | 'playback';

export interface NavigationItem {
  id: RouteId;
  labelKey: string;
  mobile: boolean;
}

export const NAVIGATION: readonly NavigationItem[] = [
  { id: 'home', labelKey: 'nav.home', mobile: true },
  { id: 'apps', labelKey: 'nav.apps', mobile: true },
  { id: 'routing', labelKey: 'nav.routing', mobile: true },
  { id: 'engine', labelKey: 'nav.engine', mobile: false },
  { id: 'settings', labelKey: 'nav.settings', mobile: true },
] as const;

const PLAYBACK_NAVIGATION_ITEM: NavigationItem = {
  id: 'playback',
  labelKey: 'nav.playback',
  mobile: true,
};

export function navigationForAudioRoute(route: AudioRouteChoice): readonly NavigationItem[] {
  return route === 'owned-file-playback'
    ? [...NAVIGATION.slice(0, 3), PLAYBACK_NAVIGATION_ITEM, ...NAVIGATION.slice(3)]
    : NAVIGATION;
}

export function routeAfterAudioRouteChoiceChanged(
  currentRoute: RouteId,
  audioRouteChoice: AudioRouteChoice,
): RouteId {
  return currentRoute === 'playback' && audioRouteChoice !== 'owned-file-playback'
    ? 'home'
    : currentRoute;
}

export function mobileNavigation(route: AudioRouteChoice = 'physical-apo'): readonly NavigationItem[] {
  return navigationForAudioRoute(route).filter((item) => item.mobile);
}
