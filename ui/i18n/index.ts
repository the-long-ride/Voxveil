import i18next, { type i18n } from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from '../../locales/en/common.json';
import enClassicDsp from '../../locales/en/classic-dsp.json';
import enSystemAudio from '../../locales/en/system-audio.json';
import es from '../../locales/es/common.json';
import esClassicDsp from '../../locales/es/classic-dsp.json';
import esSystemAudio from '../../locales/es/system-audio.json';
import fr from '../../locales/fr/common.json';
import frClassicDsp from '../../locales/fr/classic-dsp.json';
import frSystemAudio from '../../locales/fr/system-audio.json';
import ja from '../../locales/ja/common.json';
import jaClassicDsp from '../../locales/ja/classic-dsp.json';
import jaSystemAudio from '../../locales/ja/system-audio.json';
import ko from '../../locales/ko/common.json';
import koClassicDsp from '../../locales/ko/classic-dsp.json';
import koSystemAudio from '../../locales/ko/system-audio.json';
import vi from '../../locales/vi/common.json';
import viClassicDsp from '../../locales/vi/classic-dsp.json';
import viSystemAudio from '../../locales/vi/system-audio.json';
import zh from '../../locales/zh/common.json';
import zhClassicDsp from '../../locales/zh/classic-dsp.json';
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

function translation(
  common: Record<string, unknown>,
  systemAudio: Record<string, unknown>,
  classicDsp: Record<string, unknown>,
) {
  const engine = common.engine && typeof common.engine === 'object'
    ? common.engine as Record<string, unknown>
    : {};
  return { ...common, engine: { ...engine, ...classicDsp }, systemAudio };
}

export function getI18n(): i18n {
  if (instance) return instance;
  instance = i18next.createInstance();
  void instance.use(initReactI18next).init({
    lng: initialLanguage(),
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
    resources: {
      en: { translation: translation(en, enSystemAudio, enClassicDsp) },
      vi: { translation: translation(vi, viSystemAudio, viClassicDsp) },
      zh: { translation: translation(zh, zhSystemAudio, zhClassicDsp) },
      ko: { translation: translation(ko, koSystemAudio, koClassicDsp) },
      ja: { translation: translation(ja, jaSystemAudio, jaClassicDsp) },
      es: { translation: translation(es, esSystemAudio, esClassicDsp) },
      fr: { translation: translation(fr, frSystemAudio, frClassicDsp) },
    },
  });
  return instance;
}
