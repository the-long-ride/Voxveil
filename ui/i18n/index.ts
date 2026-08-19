import i18next, { type i18n } from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from '../../locales/en/common.json';
import enSystemAudio from '../../locales/en/system-audio.json';
import es from '../../locales/es/common.json';
import esSystemAudio from '../../locales/es/system-audio.json';
import fr from '../../locales/fr/common.json';
import frSystemAudio from '../../locales/fr/system-audio.json';
import ja from '../../locales/ja/common.json';
import jaSystemAudio from '../../locales/ja/system-audio.json';
import ko from '../../locales/ko/common.json';
import koSystemAudio from '../../locales/ko/system-audio.json';
import vi from '../../locales/vi/common.json';
import viSystemAudio from '../../locales/vi/system-audio.json';
import zh from '../../locales/zh/common.json';
import zhSystemAudio from '../../locales/zh/system-audio.json';
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

function translation(common: Record<string, unknown>, systemAudio: Record<string, unknown>) {
  return { ...common, systemAudio };
}

export function getI18n(): i18n {
  if (instance) return instance;
  instance = i18next.createInstance();
  void instance.use(initReactI18next).init({
    lng: initialLanguage(),
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
    resources: {
      en: { translation: translation(en, enSystemAudio) },
      vi: { translation: translation(vi, viSystemAudio) },
      zh: { translation: translation(zh, zhSystemAudio) },
      ko: { translation: translation(ko, koSystemAudio) },
      ja: { translation: translation(ja, jaSystemAudio) },
      es: { translation: translation(es, esSystemAudio) },
      fr: { translation: translation(fr, frSystemAudio) },
    },
  });
  return instance;
}
