import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BottomNavigation } from '../components/BottomNavigation';
import { SideNavigation } from '../components/SideNavigation';
import { Toggle } from '../components/Toggle';
import { AppsScreen } from '../features/apps/AppsScreen';
import { EngineScreen } from '../features/engine/EngineScreen';
import { HomeScreen } from '../features/home/HomeScreen';
import { RoutingScreen } from '../features/routing/RoutingScreen';
import { SettingsScreen } from '../features/settings/SettingsScreen';
import type { ThemeMode } from '../theme/theme';
import type { RouteId } from './navigation';
import { useVoxveilState } from './useVoxveilState';

interface AppShellProps {
  themeMode: ThemeMode;
  onThemeModeChange: (mode: ThemeMode) => void;
}

export function AppShell({ themeMode, onThemeModeChange }: AppShellProps) {
  const { t } = useTranslation();
  const [route, setRoute] = useState<RouteId>('home');
  const model = useVoxveilState();
  const screens = {
    home: <HomeScreen model={model} />,
    apps: <AppsScreen model={model} />,
    routing: <RoutingScreen model={model} />,
    engine: <EngineScreen model={model} />,
    settings: <SettingsScreen edition={model.state.edition} themeMode={themeMode} onThemeModeChange={onThemeModeChange} />,
  };

  return (
    <div className="app-shell" aria-label={t('app.name')}>
      <SideNavigation active={route} onNavigate={setRoute} />
      <div className="workspace">
        <header className="topbar">
          <div className="mobile-brand">{t('app.name')}</div>
          <div className="master-control">
            <span>{t('processing.master')}</span>
            <Toggle checked={model.state.masterEnabled} onChange={model.setMasterEnabled} label={t('processing.master')} />
          </div>
        </header>
        <main id="main-content" className="main-content">{screens[route]}</main>
      </div>
      <BottomNavigation active={route} onNavigate={setRoute} />
    </div>
  );
}
