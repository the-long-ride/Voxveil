import { useTranslation } from 'react-i18next';
import type { VoxveilModel } from '../../app/useVoxveilState';
import { ScreenIntro } from '../../components/ScreenIntro';
import { SegmentedControl } from '../../components/SegmentedControl';
import { AiModelManager } from './AiModelManager';
import type { AiModelController } from './useAiModelManager';

interface EngineScreenProps {
  model: VoxveilModel;
  aiModel: AiModelController;
}

export function EngineScreen({ model, aiModel }: EngineScreenProps) {
  const { t } = useTranslation();
  const classicOptions = [
    { value: 'music-preservation' as const, label: t('engine.musicPreservation') },
    { value: 'balanced' as const, label: t('engine.balanced') },
  ];

  return (
    <section className="screen" aria-labelledby="engine-title">
      <ScreenIntro id="engine-title" title={t('engine.title')} description={t('engine.description')} />
      <div className="engine-list">
        <article className={model.state.engine === 'dsp' ? 'engine-row is-active' : 'engine-row'}>
          <div><strong>{t('engine.dsp')}</strong><span>{t('engine.dspDescription')}</span></div><span className="mono">{t('engine.builtIn')}</span>
        </article>
        <article className={model.state.engine === 'ai' ? 'engine-row is-active' : 'engine-row'}>
          <div><strong>{t('engine.ai')}</strong><span>{t('engine.aiDescription')}</span></div><span className="mono">{aiModel.status.installed ? t('engine.modelInstalled') : t('engine.noModel')}</span>
        </article>
      </div>
      {model.state.engine !== 'ai' && (
        <div className="panel-section">
          <SegmentedControl
            label={t('engine.classicPriority')}
            value={model.state.classicSuppressionProfile}
            options={classicOptions}
            onChange={model.setClassicSuppressionProfile}
          />
          <p className="privacy-note">
            {model.state.classicSuppressionProfile === 'music-preservation'
              ? t('engine.musicPreservationDescription')
              : t('engine.balancedDescription')}
          </p>
        </div>
      )}
      <AiModelManager controller={aiModel} />
      <p className="privacy-note">{t('engine.licenseRule')}</p>
    </section>
  );
}
