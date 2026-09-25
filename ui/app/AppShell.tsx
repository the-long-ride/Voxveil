import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BottomNavigation } from '../components/BottomNavigation';
import { SideNavigation } from '../components/SideNavigation';
import { Toggle } from '../components/Toggle';
import { AppsScreen } from '../features/apps/AppsScreen';
import { EngineScreen } from '../features/engine/EngineScreen';
import { useAiModelManager } from '../features/engine/useAiModelManager';
import { HomeScreen } from '../features/home/HomeScreen';
import { PlaybackScreen } from '../features/playback/PlaybackScreen';
import { RoutingScreen } from '../features/routing/RoutingScreen';
import { SettingsScreen } from '../features/settings/SettingsScreen';
import type { ThemeMode } from '../theme/theme';
import { routeAfterAudioRouteChoiceChanged, type RouteId } from './navigation';
import { useVoxveilState } from './useVoxveilState';

interface AppShellProps {
  themeMode: ThemeMode;
  onThemeModeChange: (mode: ThemeMode) => void;
}

export function AppShell({ themeMode, onThemeModeChange }: AppShellProps) {
  const { t } = useTranslation();
  const [route, setRoute] = useState<RouteId>('home');
  const model = useVoxveilState();
  const aiModel = useAiModelManager(() => model.setEngine('auto'));
  useEffect(() => {
    setRoute((current) => routeAfterAudioRouteChoiceChanged(current, model.state.audioRouteChoice));
  }, [model.state.audioRouteChoice]);
  const screens = {
    home: <HomeScreen model={model} aiModelReady={aiModel.status.installed && aiModel.status.runtimeAvailable} />,
    apps: <AppsScreen model={model} />,
    routing: <RoutingScreen model={model} />,
    engine: <EngineScreen model={model} aiModel={aiModel} />,
    playback: <PlaybackScreen
      playback={model.state.playback}
      outputs={model.physicalOutputs}
      selectedOutputId={model.state.physicalOutputEndpointId}
      operationError={model.playbackOperationError}
      onOpen={model.openAudioFile}
      onPause={model.pausePlayback}
      onResume={model.resumePlayback}
      onStop={model.stopPlayback}
      onSeek={model.seekPlayback}
      onSelectOutput={model.selectPhysicalOutput}
    />,
    settings: <SettingsScreen
      edition={model.state.edition}
      windowsAudioRoutesAvailable={model.state.windowsAudioRoutesAvailable}
      audioRouteChoice={model.state.audioRouteChoice}
      audioRouteError={model.audioRouteError ?? model.state.audioRouteError}
      onAudioRouteChange={model.setAudioRoute}
      themeMode={themeMode}
      onThemeModeChange={onThemeModeChange}
    />,
  };

  return (
    <div className="app-shell" aria-label={t('app.name')}>
      <SideNavigation active={route} onNavigate={setRoute} audioRouteChoice={model.state.audioRouteChoice} />
      <div className="workspace">
        <header className="topbar">
          <div className="mobile-brand">{t('app.name')}</div>
          <div className="master-control">
            <span>{t('processing.master')}</span>
            <Toggle
              checked={model.state.masterEnabled}
              onChange={model.setMasterEnabled}
              label={t('processing.master')}
              disabled={!model.canStartProcessing}
            />
          </div>
        </header>
        <main id="main-content" className="main-content">
          {model.processingOperationError && (
            <div className="backend-notice" role="alert">
              <strong>{t('processing.master')}</strong>
              <span>{model.processingOperationError}</span>
            </div>
          )}
          {screens[route]}
        </main>
      </div>
      <BottomNavigation active={route} onNavigate={setRoute} audioRouteChoice={model.state.audioRouteChoice} />
    </div>
  );
}
