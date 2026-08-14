import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AiModelController } from './useAiModelManager';

function progressPercent(controller: AiModelController): number | null {
  const { progress } = controller;
  if (!progress?.totalBytes || progress.totalBytes <= 0) return null;
  return Math.min(100, Math.round((progress.downloadedBytes / progress.totalBytes) * 100));
}

export function AiModelManager({ controller }: { controller: AiModelController }) {
  const { t } = useTranslation();
  const [consentOpen, setConsentOpen] = useState(false);
  const [agreed, setAgreed] = useState(false);
  const percent = progressPercent(controller);
  const { status } = controller;

  const beginInstall = () => {
    if (!agreed) return;
    setConsentOpen(false);
    setAgreed(false);
    void controller.install(true);
  };

  return (
    <section className="model-manager" aria-labelledby="ai-model-title">
      <div className="model-manager-header">
        <div><strong id="ai-model-title">{status.displayName}</strong><span>{status.license} · ~{status.approximateSizeMb} MB</span></div>
        <span className="mono">{status.installed ? t('engine.modelInstalled') : t('engine.noModel')}</span>
      </div>
      <p>{t('engine.modelStorage')}</p>
      <p className="meta">{t('engine.modelSource', { source: status.source })}</p>
      {status.installed && !status.runtimeAvailable && <p className="meta">{t('engine.runtimePending')}</p>}
      {controller.error && <p className="model-error" role="alert">{t(`engine.modelError.${controller.error}`)}</p>}
      {controller.busy && <div className="download-progress" aria-label={t('engine.downloading')}>
        <div className="download-progress-track"><span style={{ width: `${percent ?? 12}%` }} /></div>
        <span className="mono">{percent === null ? t('engine.downloading') : `${percent}%`}</span>
      </div>}
      <div className="model-actions">
        {!status.installed && <button className="action-button" type="button" disabled={!controller.native || !status.downloadAvailable || controller.busy} onClick={() => setConsentOpen(true)}>{t('engine.installModel')}</button>}
        {status.installed && <button className="action-button" type="button" disabled={controller.busy} onClick={() => void controller.remove()}>{t('engine.removeModel')}</button>}
      </div>

      {consentOpen && <div className="dialog-backdrop" role="presentation">
        <section className="consent-dialog" role="dialog" aria-modal="true" aria-labelledby="model-consent-title">
          <h2 id="model-consent-title">{t('engine.consentTitle')}</h2>
          <p>{t('engine.consentDescription', { name: status.displayName, size: status.approximateSizeMb })}</p>
          <p>{t('engine.networkNotice')}</p>
          <label className="consent-check"><input type="checkbox" checked={agreed} onChange={(event) => setAgreed(event.target.checked)} /> <span>{t('engine.consentAgreement', { license: status.license })}</span></label>
          <div className="dialog-actions">
            <button className="action-button is-subtle" type="button" disabled={controller.busy} onClick={() => { setConsentOpen(false); setAgreed(false); }}>{t('common.cancel')}</button>
            <button className="action-button" type="button" disabled={!agreed || controller.busy} onClick={beginInstall}>{t('engine.agreeAndDownload')}</button>
          </div>
        </section>
      </div>}
    </section>
  );
}
