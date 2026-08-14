import { useTranslation } from 'react-i18next';
import type { NavigationItem, RouteId } from '../app/navigation';

interface NavigationButtonsProps {
  items: readonly NavigationItem[];
  active: RouteId;
  onNavigate: (route: RouteId) => void;
}

export function NavigationButtons({ items, active, onNavigate }: NavigationButtonsProps) {
  const { t } = useTranslation();
  return items.map((item) => (
    <button
      type="button"
      key={item.id}
      className={active === item.id ? 'is-active' : ''}
      onClick={() => onNavigate(item.id)}
    >
      {t(item.labelKey)}
    </button>
  ));
}
