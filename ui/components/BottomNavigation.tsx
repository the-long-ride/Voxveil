import { useTranslation } from 'react-i18next';
import { mobileNavigation, type RouteId } from '../app/navigation';
import type { AudioRouteChoice } from '../lib/types';
import { NavigationButtons } from './NavigationButtons';

interface BottomNavigationProps {
  active: RouteId;
  onNavigate: (route: RouteId) => void;
  audioRouteChoice: AudioRouteChoice;
}

export function BottomNavigation({ active, onNavigate, audioRouteChoice }: BottomNavigationProps) {
  const { t } = useTranslation();
  return (
    <nav className="bottom-nav" aria-label={t('nav.mobile')} style={{ gridTemplateColumns: `repeat(${mobileNavigation(audioRouteChoice).length}, minmax(0, 1fr))` }}>
      <NavigationButtons items={mobileNavigation(audioRouteChoice)} active={active} onNavigate={onNavigate} />
    </nav>
  );
}
