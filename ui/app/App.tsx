import { useState } from 'react';
import { I18nextProvider } from 'react-i18next';
import { ThemeProvider } from '../components/ThemeProvider';
import { getI18n } from '../i18n';
import { loadThemeMode, saveThemeMode } from '../theme/theme-storage';
import type { ThemeMode } from '../theme/theme';
import { AppShell } from './AppShell';

export function App() {
  const [themeMode, setThemeModeState] = useState<ThemeMode>(loadThemeMode);
  const setThemeMode = (mode: ThemeMode) => {
    saveThemeMode(mode);
    setThemeModeState(mode);
  };
  return (
    <I18nextProvider i18n={getI18n()}>
      <ThemeProvider mode={themeMode}>
        <AppShell themeMode={themeMode} onThemeModeChange={setThemeMode} />
      </ThemeProvider>
    </I18nextProvider>
  );
}
