import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ScreenIntro } from '../../components/ScreenIntro';
import { RangeControl } from '../../components/RangeControl';
import { SegmentedControl } from '../../components/SegmentedControl';
import type { VoxveilModel } from '../../app/useVoxveilState';
import type { ProcessingBackendStatus, ProcessingLoad } from '../../lib/types';

const LOAD_KEYS: Record<ProcessingLoad, string> = {
  idle: 'status.idleLoad',
  low: 'status.lowLoad',
  medium: 'status.mediumLoad',
  high: 'status.highLoad',
};

function loadLabelKey(load: ProcessingLoad): string {
  return LOAD_KEYS[load];
}

const BACKEND_KEYS: Record<Exclude<ProcessingBackendStatus, 'ready'>, string> = {
  'component-required': 'processing.backendComponentRequired',
  'routing-required': 'processing.backendRoutingRequired',
  unsupported: 'processing.backendUnsupported',
  faulted: 'processing.backendFaulted',
};

function backendLabelKey(status: Exclude<ProcessingBackendStatus, 'ready'>): string {
  return BACKEND_KEYS[status];
}

export function HomeScreen({ model, aiModelReady }: { model: VoxveilModel; aiModelReady: boolean }) {
  const { t } = useTranslation();
  const { state } = model;
  const [installing, setInstalling] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);

  const installSystemAudio = async () => {
    setInstalling(true);
    setInstallError(null);
    try {
      await model.installSystemAudioComponent();
    } catch (error) {
      setInstallError(error instanceof Error ? error.message : String(error));
    } finally {
      setInstalling(false);
    }
  };

  return (
    <section className="screen" aria-labelledby="home-title">
      <ScreenIntro id="home-title" title={t('home.title')} description={t('home.description')} />

      {state.backendStatus !== 'ready' && (
        <div className="backend-notice" role="status">
          <strong>{t('processing.backendUnavailable')}</strong>
          <span>{t(backendLabelKey(state.backendStatus))}</span>
          {state.backendStatus === 'component-required' && (
            <div className="model-actions">
              <button type="button" className="action-button" disabled={installing} onClick={() => void installSystemAudio()}>
                {installing
                  ? t('processing.installingSystemAudio', { defaultValue: 'Installing…' })
                  : t('processing.installSystemAudio', { defaultValue: 'Install system audio component' })}
              </button>
            </div>
          )}
          {installError && <span className="model-error">{installError}</span>}
        </div>
      )}

      <div className="primary-controls">
        <RangeControl id="vocals" label={t('processing.vocals')} value={state.vocalLevel} valueLabel={`${state.vocalLevel}%`} onChange={model.setVocalLevel} />
        <RangeControl id="quality" label={t('processing.quality')} value={state.quality} startLabel={t('processing.lowLatency')} endLabel={t('processing.highQuality')} onChange={model.setQuality} />
      </div>

      <div className="control-grid">
        <SegmentedControl label={t('processing.mode')} value={state.processingMode} onChange={model.setProcessingMode} options={[
          { value: 'all', label: t('processing.allOutput') },
          { value: 'per-app', label: t('processing.perApp'), disabled: !state.perAppProcessingAvailable },
        ]} />
        <SegmentedControl label={t('processing.engine')} value={state.engine} onChange={model.setEngine} options={[
          { value: 'auto', label: t('engine.auto') },
          { value: 'dsp', label: t('engine.dsp') },
          { value: 'ai', label: t('engine.ai'), disabled: !aiModelReady },
        ]} />
      </div>

      <div className="status-strip" aria-label={t('status.current')}>
        <div><span>{t('status.latency')}</span><strong className="mono">~{state.estimatedLatencyMs} ms</strong></div>
        <div><span>{t('status.load')}</span><strong>{t(loadLabelKey(state.load))}</strong></div>
        <div><span>{t('status.output')}</span><strong>{state.physicalOutput}</strong></div>
      </div>
    </section>
  );
}
