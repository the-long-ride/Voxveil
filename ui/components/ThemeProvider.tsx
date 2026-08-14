import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { resolveTheme, type ThemeMode } from '../theme/theme';

interface ThemeProviderProps {
  mode: ThemeMode;
  children: ReactNode;
}

function systemPrefersDark(): boolean {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

export function ThemeProvider({ mode, children }: ThemeProviderProps) {
  const [systemDark, setSystemDark] = useState(systemPrefersDark);
  const resolved = useMemo(() => resolveTheme(mode, systemDark), [mode, systemDark]);

  useEffect(() => {
    const media = window.matchMedia?.('(prefers-color-scheme: dark)');
    if (!media) return undefined;
    const update = () => setSystemDark(media.matches);
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = resolved;
    document.documentElement.style.colorScheme = resolved;
  }, [resolved]);

  return children;
}
