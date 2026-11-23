import { useTranslation } from 'react-i18next';
import type { AiPreferences } from '../../lib/types';
import { Toggle } from '../../components/Toggle';
import { AiModelManager } from '../engine/AiModelManager';
import { useAiModelManager } from '../engine/useAiModelManager';

export function AiModelsSettings({ preferences, onChange, onActiveModelRemoved }: { preferences: AiPreferences; onChange: (next: AiPreferences) => void; onActiveModelRemoved?: () => void }) {
  const { t } = useTranslation();
  const spleeter = useAiModelManager(() => { if (preferences.model === 'spleeter-2stems-fp16') onActiveModelRemoved?.(); }, 'spleeter-2stems-fp16');
  const mdx = useAiModelManager(() => { if (preferences.model === 'mdx-inst-hq3') onActiveModelRemoved?.(); }, 'mdx-inst-hq3');
  return <section className="settings-group" aria-label={t('aiPlayer.models')}>
    <h2>{t('aiPlayer.models')}</h2>
    <p>{t('aiPlayer.changesStop')}</p>
    {[spleeter, mdx].map((controller) => <div key={controller.status.id}>
      <label>
        <input type="radio" name="ai-model" value={controller.status.id}
          checked={preferences.model === controller.status.id}
          onChange={() => onChange({ model: controller.status.id as AiPreferences['model'], systemWide: controller.status.id === 'spleeter-2stems-fp16' && preferences.systemWide })} />
        {t('aiPlayer.choose', { name: controller.status.displayName })}
      </label>
      <AiModelManager controller={controller} />
    </div>)}
    <div className="settings-row">
      <span><strong>{t('aiPlayer.systemWide')}</strong><small>{t('aiPlayer.delay')}</small></span>
      <Toggle label={t('aiPlayer.systemWide')} checked={preferences.systemWide}
        disabled={preferences.model !== 'spleeter-2stems-fp16'} onChange={(systemWide) => onChange({ ...preferences, systemWide })} />
    </div>
    {preferences.model === 'mdx-inst-hq3' && <p>{t('aiPlayer.mdxOffline')}</p>}
  </section>;
}
