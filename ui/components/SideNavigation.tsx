import { useTranslation } from 'react-i18next';
import { NAVIGATION, type RouteId } from '../app/navigation';
import { NavigationButtons } from './NavigationButtons';

interface SideNavigationProps {
  active: RouteId;
  onNavigate: (route: RouteId) => void;
}

export function SideNavigation({ active, onNavigate }: SideNavigationProps) {
  const { t } = useTranslation();
  return (
    <aside className="side-nav" aria-label={t('nav.primary')}>
      <div className="brand">{t('app.name')}</div>
      <nav><NavigationButtons items={NAVIGATION} active={active} onNavigate={onNavigate} /></nav>
      <div className="side-foot">{t('privacy.localOnly')}</div>
    </aside>
  );
}
