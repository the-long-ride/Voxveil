import { useTranslation } from 'react-i18next';
import { ScreenIntro } from '../../components/ScreenIntro';
import type { VoxveilModel } from '../../app/useVoxveilState';
import { SegmentedControl } from '../../components/SegmentedControl';

export function RoutingScreen({ model }: { model: VoxveilModel }) {
  const { t } = useTranslation();
  const { state } = model;
  return (
    <section className="screen" aria-labelledby="routing-title">
      <ScreenIntro id="routing-title" title={t('routing.title')} description={t('routing.description')} />
      <div className="section-block">
        <SegmentedControl label={t('routing.outputMode')} value={state.outputMode} onChange={model.setOutputMode} options={[
          { value: 'physical', label: t('routing.physical') },
          { value: 'virtual', label: t('routing.virtual'), disabled: !state.virtualOutputAvailable },
          { value: 'both', label: t('routing.both'), disabled: !state.virtualOutputAvailable },
        ]} />
      </div>
      <div className="list">
        <div className="list-row"><div><strong>{t('routing.physical')}</strong><span>{state.physicalOutput}</span></div><span className="status-dot is-on">{t('common.ready')}</span></div>
        <div className="list-row"><div><strong>{t('routing.virtual')}</strong><span>{t('routing.virtualName')}</span></div><span className="status-dot is-on">{state.virtualOutputAvailable ? t('common.ready') : t('common.unavailable')}</span></div>
      </div>
    </section>
  );
}
