import { useTranslation } from 'react-i18next';
import type { SystemAudioEndpoint } from '../../lib/types';

export interface SystemAudioEndpointsProps {
  endpoints: SystemAudioEndpoint[];
  busy: boolean;
  installBusyId: string | null;
  error: string | null;
  onRefresh: () => void;
  onInstall: (endpointId: string) => void;
  onInstallAll: () => void;
}

function statusLabel(endpoint: SystemAudioEndpoint, readyLabel: string, unavailableLabel: string): string {
  switch (endpoint.status) {
    case 'ready': return readyLabel;
    case 'installable': return 'Ready to install';
    case 'component-required': return 'Signed component required';
    case 'ambiguous': return 'Ambiguous driver topology';
    case 'unsupported': return unavailableLabel;
  }
}

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
    <section className="system-audio-panel" aria-label="Windows System Audio">
      <div className="system-audio-header">
        <div>
          <strong>Windows System Audio</strong>
          <span>Detected playback outputs</span>
        </div>
        <div className="system-audio-actions">
          {installableCount >= 2 && (
            <button className="action-button is-subtle" type="button" disabled={Boolean(installBusyId)} onClick={onInstallAll}>
              Install all compatible outputs
            </button>
          )}
          <button className="action-button is-subtle" type="button" disabled={busy || Boolean(installBusyId)} onClick={onRefresh}>
            {busy ? 'Refreshing…' : 'Refresh'}
          </button>
        </div>
      </div>

      {endpoints.length === 0 && <span className="meta">{busy ? 'Detecting playback outputs…' : 'No playback outputs detected.'}</span>}
      <div className="system-audio-list">
        {endpoints.map((endpoint) => (
          <div className="system-audio-row" data-testid={`system-audio-endpoint-${endpoint.endpointId}`} key={endpoint.endpointId}>
            <div className="system-audio-identity">
              <div className="system-audio-name">
                <strong>{endpoint.displayName}</strong>
                {endpoint.isDefault && <span className="endpoint-badge">Default</span>}
              </div>
              {endpoint.adapterName && <span>{endpoint.adapterName}</span>}
              {endpoint.detail && <small>{endpoint.detail}</small>}
            </div>
            <div className="system-audio-state">
              <span className={endpoint.status === 'ready' ? 'status-dot is-on' : 'status-dot'}>
                {statusLabel(endpoint, t('common.ready'), t('common.unavailable'))}
              </span>
              {endpoint.status === 'installable' && (
                <button
                  className="action-button"
                  type="button"
                  disabled={Boolean(installBusyId)}
                  onClick={() => onInstall(endpoint.endpointId)}
                >
                  {installBusyId === endpoint.endpointId ? 'Installing…' : 'Install'}
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
