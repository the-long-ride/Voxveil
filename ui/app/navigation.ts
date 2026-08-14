export type RouteId = 'home' | 'apps' | 'routing' | 'engine' | 'settings';

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

export function mobileNavigation(): readonly NavigationItem[] {
  return NAVIGATION.filter((item) => item.mobile);
}
