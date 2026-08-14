import { useTranslation } from 'react-i18next';
import { mobileNavigation, type RouteId } from '../app/navigation';
import { NavigationButtons } from './NavigationButtons';

interface BottomNavigationProps {
  active: RouteId;
  onNavigate: (route: RouteId) => void;
}

export function BottomNavigation({ active, onNavigate }: BottomNavigationProps) {
  const { t } = useTranslation();
  return (
    <nav className="bottom-nav" aria-label={t('nav.mobile')}>
      <NavigationButtons items={mobileNavigation()} active={active} onNavigate={onNavigate} />
    </nav>
  );
}
