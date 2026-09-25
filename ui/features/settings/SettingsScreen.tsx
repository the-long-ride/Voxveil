import { useTranslation } from 'react-i18next';
import { ScreenIntro } from '../../components/ScreenIntro';
import { SegmentedControl } from '../../components/SegmentedControl';
import { saveLanguage } from '../../i18n/language-storage';
import { SUPPORTED_LANGUAGES, type SupportedLanguage } from '../../i18n/languages';
import type { ThemeMode } from '../../theme/theme';
import type { AudioRouteChoice } from '../../lib/types';

interface SettingsScreenProps {
  edition?: 'standard' | 'pro-system';
  windowsAudioRoutesAvailable: boolean;
  audioRouteChoice: AudioRouteChoice;
  audioRouteError: string | null;
  onAudioRouteChange: (route: AudioRouteChoice) => void | Promise<void>;
  themeMode: ThemeMode;
  onThemeModeChange: (mode: ThemeMode) => void;
}

export function SettingsScreen({
  edition = 'standard', windowsAudioRoutesAvailable, audioRouteChoice, audioRouteError, onAudioRouteChange,
  themeMode, onThemeModeChange,
}: SettingsScreenProps) {
  const { t, i18n } = useTranslation();
  const language = (i18n.resolvedLanguage ?? 'en') as SupportedLanguage;
  const changeLanguage = (next: string) => {
    const language = next as SupportedLanguage;
    saveLanguage(language);
    void i18n.changeLanguage(language);
  };
  return (
    <section className="screen" aria-labelledby="settings-title">
      <ScreenIntro id="settings-title" title={t('settings.title')} description={t('settings.description')} />
      <div className="settings-group">
        <div className="settings-row">
          <span><strong>{t('settings.edition')}</strong></span>
          <span className="mono">{t(edition === 'pro-system' ? 'settings.editionProSystem' : 'settings.editionStandard')}</span>
        </div>
        {windowsAudioRoutesAvailable && <div className="settings-row settings-row-stacked">
          <label htmlFor="audio-route">
            <strong>{t('systemAudio.processingRouteLabel')}</strong>
            <small>{t('systemAudio.processingRouteDescription')}</small>
          </label>
          <select
            id="audio-route"
            value={audioRouteChoice}
            aria-describedby="audio-route-description"
            onChange={(event) => void onAudioRouteChange(event.currentTarget.value as AudioRouteChoice)}
          >
            <option value="physical-apo">{t('systemAudio.physicalApoRoute')}</option>
            <option value="legacy-automatic">{t('systemAudio.legacyAutomaticRoute')}</option>
            <option value="owned-file-playback">{t('systemAudio.ownedPlaybackRoute')}</option>
          </select>
          <small id="audio-route-description">
            {t(`systemAudio.${audioRouteChoice === 'physical-apo' ? 'physicalApoDescription' : audioRouteChoice === 'owned-file-playback' ? 'ownedPlaybackDescription' : 'legacyAutomaticDescription'}`)}
          </small>
          {(audioRouteError) && <p className="settings-error" role="alert">{audioRouteError}</p>}
        </div>}
        <div className="settings-row settings-row-stacked">
          <span><strong>{t('settings.appearance')}</strong><small>{t('settings.appearanceDescription')}</small></span>
          <SegmentedControl label={t('settings.appearance')} value={themeMode} onChange={onThemeModeChange} options={[
            { value: 'system', label: t('settings.themeSystem') },
            { value: 'light', label: t('settings.themeLight') },
            { value: 'dark', label: t('settings.themeDark') },
          ]} />
        </div>
        <label className="settings-row" htmlFor="language">
          <span><strong>{t('settings.language')}</strong><small>{t('settings.languageDescription')}</small></span>
          <select id="language" aria-label={t('settings.language')} value={language} onChange={(event) => changeLanguage(event.currentTarget.value)}>
            {SUPPORTED_LANGUAGES.map((code) => <option key={code} value={code}>{t(`languages.${code}`)}</option>)}
          </select>
        </label>
        <div className="settings-row">
          <span><strong>{t('settings.privacy')}</strong><small>{t('settings.privacyDescription')}</small></span>
          <span className="status-dot is-on">{t('settings.local')}</span>
        </div>
      </div>
    </section>
  );
}
