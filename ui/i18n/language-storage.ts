import { readLocalSetting, writeLocalSetting } from '../lib/safe-storage';
import { SUPPORTED_LANGUAGES, type SupportedLanguage } from './languages';

const LANGUAGE_KEY = 'voxveil.language';

export function loadStoredLanguage(): SupportedLanguage | null {
  const value = readLocalSetting(LANGUAGE_KEY) as SupportedLanguage | null;
  return value && SUPPORTED_LANGUAGES.includes(value) ? value : null;
}

export function saveLanguage(language: SupportedLanguage): void {
  writeLocalSetting(LANGUAGE_KEY, language);
}
