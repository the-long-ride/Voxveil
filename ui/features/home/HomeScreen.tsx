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

type InstallErrorDetails = {
  message: string;
  exitCode: number | null;
  stdout: string;
  stderr: string;
};

function normalizeInstallError(error: unknown): InstallErrorDetails {
  if (error && typeof error === 'object') {
    const value = error as Record<string, unknown>;
    return {
      message: typeof value.message === 'string' ? value.message : 'Windows system-audio installation failed',
      exitCode: typeof value.exitCode === 'number' ? value.exitCode : null,
      stdout: typeof value.stdout === 'string' ? value.stdout : '',
      stderr: typeof value.stderr === 'string' ? value.stderr : '',
    };
  }
  if (error instanceof Error) {
    return { message: error.message, exitCode: null, stdout: '', stderr: '' };
  }
  if (typeof error === 'string') {
    try {
      return normalizeInstallError(JSON.parse(error) as unknown);
    } catch {
      return { message: error, exitCode: null, stdout: '', stderr: '' };
    }
  }
  return { message: String(error), exitCode: null, stdout: '', stderr: '' };
}

function formatInstallErrorDetails(error: InstallErrorDetails): string {
  const sections = [`Message: ${error.message}`];
  if (error.exitCode !== null) sections.push(`Exit code: ${error.exitCode}`);
  if (error.stdout) sections.push(`stdout:\n${error.stdout}`);
  if (error.stderr) sections.push(`stderr:\n${error.stderr}`);
  return sections.join('\n\n');
}

export function HomeScreen({ model, aiModelReady }: { model: VoxveilModel; aiModelReady: boolean }) {
  const { t } = useTranslation();
  const { state } = model;
  const [installing, setInstalling] = useState(false);
  const [installError, setInstallError] = useState<InstallErrorDetails | null>(null);
  const [showInstallErrorDetails, setShowInstallErrorDetails] = useState(false);
  const [detailsCopied, setDetailsCopied] = useState(false);

  const installSystemAudio = async () => {
    setInstalling(true);
    setInstallError(null);
    setShowInstallErrorDetails(false);
    setDetailsCopied(false);
    try {
      await model.installSystemAudioComponent();
    } catch (error) {
      const normalized = normalizeInstallError(error);
      setInstallError(normalized);
      setShowInstallErrorDetails(true);
    } finally {
      setInstalling(false);
    }
  };

  const copyInstallErrorDetails = async () => {
    if (!installError) return;
    await navigator.clipboard.writeText(formatInstallErrorDetails(installError));
    setDetailsCopied(true);
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
              <button
                type="button"
                className="action-button is-subtle install-error-details-button"
                disabled={!installError}
                onClick={() => setShowInstallErrorDetails(true)}
              >
                {t('processing.viewErrorDetails', { defaultValue: 'View details' })}
              </button>
            </div>
          )}
          {installError && (
            <div className="install-error-summary">
              <span className="model-error">{installError.message}</span>
            </div>
          )}
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

      {showInstallErrorDetails && installError && (
        <div className="dialog-backdrop" role="presentation" onMouseDown={() => setShowInstallErrorDetails(false)}>
          <div
            className="consent-dialog install-error-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="install-error-title"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <div className="install-error-dialog-heading">
              <div>
                <h2 id="install-error-title">
                  {t('processing.installErrorTitle', { defaultValue: 'System audio installation failed' })}
                </h2>
                <p>{t('processing.installErrorDescription', { defaultValue: 'Voxveil captured the installer output below.' })}</p>
              </div>
            </div>
            <pre className="install-error-output">{formatInstallErrorDetails(installError)}</pre>
            <div className="dialog-actions">
              <button type="button" className="action-button is-subtle" onClick={() => void copyInstallErrorDetails()}>
                {detailsCopied
                  ? t('processing.errorDetailsCopied', { defaultValue: 'Copied' })
                  : t('processing.copyErrorDetails', { defaultValue: 'Copy details' })}
              </button>
              <button type="button" className="action-button" onClick={() => setShowInstallErrorDetails(false)}>
                {t('common.close', { defaultValue: 'Close' })}
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
