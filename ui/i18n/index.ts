import i18next, { type i18n } from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from '../../locales/en/common.json';
import es from '../../locales/es/common.json';
import fr from '../../locales/fr/common.json';
import ja from '../../locales/ja/common.json';
import ko from '../../locales/ko/common.json';
import vi from '../../locales/vi/common.json';
import zh from '../../locales/zh/common.json';
import { loadStoredLanguage } from './language-storage';
import { SUPPORTED_LANGUAGES, type SupportedLanguage } from './languages';

let instance: i18n | null = null;

function initialLanguage(): SupportedLanguage {
  try {
    const stored = loadStoredLanguage();
    if (stored) return stored;
    const browser = globalThis.navigator?.language?.split('-')[0] as SupportedLanguage | undefined;
    if (browser && SUPPORTED_LANGUAGES.includes(browser)) return browser;
  } catch {
    // Hardened webviews may block storage or navigator access.
  }
  return 'en';
}

export function getI18n(): i18n {
  if (instance) return instance;
  instance = i18next.createInstance();
  void instance.use(initReactI18next).init({
    lng: initialLanguage(),
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
    resources: {
      en: { translation: en }, vi: { translation: vi }, zh: { translation: zh },
      ko: { translation: ko }, ja: { translation: ja }, es: { translation: es },
      fr: { translation: fr },
    },
  });
  return instance;
}
