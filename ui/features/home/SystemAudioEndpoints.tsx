import { useTranslation } from 'react-i18next';
import type { SystemAudioEndpoint, SystemAudioEndpointStatus } from '../../lib/types';

export interface SystemAudioEndpointsProps {
  endpoints: SystemAudioEndpoint[];
  busy: boolean;
  installBusyId: string | null;
  error: string | null;
  onRefresh: () => void;
  onInstall: (endpointId: string) => void;
  onInstallAll: () => void;
}

const STATUS_KEYS: Record<SystemAudioEndpointStatus, string> = {
  ready: 'common.ready',
  installable: 'systemAudio.readyToInstall',
  'component-required': 'systemAudio.signedComponentRequired',
  ambiguous: 'systemAudio.ambiguous',
  unsupported: 'common.unavailable',
};

export function SystemAudioEndpoints({
  endpoints,
  busy,
  installBusyId,
  error,
  onRefresh,
  onInstall,
  onInstallAll,
}: SystemAudioEndpointsProps) {
  const { t } = useTranslation();
  const installableCount = endpoints.filter((endpoint) => endpoint.status === 'installable').length;

  return (
    <section className="system-audio-panel" aria-label={t('systemAudio.title')}>
      <div className="system-audio-header">
        <div>
          <strong>{t('systemAudio.title')}</strong>
          <span>{t('systemAudio.description')}</span>
        </div>
        <div className="system-audio-actions">
          {installableCount >= 2 && (
            <button className="action-button is-subtle" type="button" disabled={Boolean(installBusyId)} onClick={onInstallAll}>
              {t('systemAudio.installAll')}
            </button>
          )}
          <button className="action-button is-subtle" type="button" disabled={busy || Boolean(installBusyId)} onClick={onRefresh}>
            {t(busy ? 'systemAudio.refreshing' : 'systemAudio.refresh')}
          </button>
        </div>
      </div>

      {endpoints.length === 0 && <span className="meta">{t(busy ? 'systemAudio.detecting' : 'systemAudio.none')}</span>}
      <div className="system-audio-list">
        {endpoints.map((endpoint) => (
          <div className="system-audio-row" data-testid={`system-audio-endpoint-${endpoint.endpointId}`} key={endpoint.endpointId}>
            <div className="system-audio-identity">
              <div className="system-audio-name">
                <strong>{endpoint.displayName}</strong>
                {endpoint.isDefault && <span className="endpoint-badge">{t('systemAudio.default')}</span>}
              </div>
              {endpoint.adapterName && <span>{endpoint.adapterName}</span>}
              {endpoint.detail && <small>{endpoint.detail}</small>}
            </div>
            <div className="system-audio-state">
              <span className={endpoint.status === 'ready' ? 'status-dot is-on' : 'status-dot'}>
                {t(STATUS_KEYS[endpoint.status])}
              </span>
              {endpoint.status === 'installable' && (
                <button
                  className="action-button"
                  type="button"
                  disabled={Boolean(installBusyId)}
                  onClick={() => onInstall(endpoint.endpointId)}
                >
                  {t(installBusyId === endpoint.endpointId ? 'systemAudio.installing' : 'systemAudio.install')}
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
      {error && <span className="model-error system-audio-error" role="alert">{error}</span>}
    </section>
  );
}
