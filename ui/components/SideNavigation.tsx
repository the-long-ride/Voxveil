import { useTranslation } from 'react-i18next';
import { navigationForAudioRoute, type RouteId } from '../app/navigation';
import type { AudioRouteChoice } from '../lib/types';
import { NavigationButtons } from './NavigationButtons';

interface SideNavigationProps {
  active: RouteId;
  onNavigate: (route: RouteId) => void;
  audioRouteChoice: AudioRouteChoice;
}

export function SideNavigation({ active, onNavigate, audioRouteChoice }: SideNavigationProps) {
  const { t } = useTranslation();
  return (
    <aside className="side-nav" aria-label={t('nav.primary')}>
      <div className="brand">{t('app.name')}</div>
      <nav><NavigationButtons items={navigationForAudioRoute(audioRouteChoice)} active={active} onNavigate={onNavigate} /></nav>
      <div className="side-foot">{t('privacy.localOnly')}</div>
    </aside>
  );
}
