import { readLocalSetting, writeLocalSetting } from '../lib/safe-storage';
import type { ThemeMode } from './theme';

const KEY = 'voxveil.theme';
const MODES = new Set<ThemeMode>(['system', 'light', 'dark']);

export function loadThemeMode(): ThemeMode {
  const value = readLocalSetting(KEY) as ThemeMode | null;
  return value && MODES.has(value) ? value : 'system';
}

export function saveThemeMode(mode: ThemeMode): void {
  writeLocalSetting(KEY, mode);
}
