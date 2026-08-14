import { useTranslation } from 'react-i18next';
import { ScreenIntro } from '../../components/ScreenIntro';
import type { VoxveilModel } from '../../app/useVoxveilState';
import { Toggle } from '../../components/Toggle';

export function AppsScreen({ model }: { model: VoxveilModel }) {
  const { t } = useTranslation();
  return (
    <section className="screen" aria-labelledby="apps-title">
      <ScreenIntro id="apps-title" title={t('apps.title')} description={t('apps.description')} />
      <div className="list" role="list">
        {model.state.apps.map((app) => (
          <div className="list-row" role="listitem" key={app.id}>
            <div>
              <strong>{app.name}</strong>
              <span>{app.bypassReason ? t('apps.communicationBypass') : t(`apps.category.${app.category}`)}</span>
            </div>
            <Toggle checked={app.enabled} onChange={(enabled) => model.setAppEnabled(app.id, enabled)} label={`${app.name} ${t('common.enabled')}`} disabled={app.bypassReason === 'communication'} />
          </div>
        ))}
      </div>
    </section>
  );
}
