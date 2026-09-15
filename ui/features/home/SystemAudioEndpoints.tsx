import { useTranslation } from 'react-i18next';
import type {
  AudioOutput,
  ProcessingBackendStatus,
  SystemAudioEndpoint,
  SystemAudioEndpointStatus,
  WindowsInterceptionKind,
} from '../../lib/types';

export interface SystemAudioEndpointsProps {
  endpoints: SystemAudioEndpoint[];
  backendStatus: ProcessingBackendStatus;
  backendKind: WindowsInterceptionKind | null;
  physicalOutputs: AudioOutput[];
  selectedPhysicalOutputId: string | null;
  busy: boolean;
  installBusyId: string | null;
  error: string | null;
  onRefresh: () => void;
  onInstall: (endpointId: string) => void;
  onInstallAll: () => void;
  onSelectPhysicalOutput: (endpointId: string) => void;
  onOpenSoundSettings: () => void;
  onGetVbCable: () => void;
}

const STATUS_KEYS: Record<SystemAudioEndpointStatus, string> = {
  ready: 'common.ready',
  installable: 'systemAudio.readyToInstall',
  'component-required': 'systemAudio.signedComponentRequired',
  ambiguous: 'systemAudio.ambiguous',
  unsupported: 'common.unavailable',
};

function backendLabel(kind: WindowsInterceptionKind | null): string | null {
  switch (kind) {
    case 'apo': return 'Voxveil APO';
    case 'vb-cable-relay': return 'VB-CABLE';
    case 'voxveil-cable-relay': return 'Voxveil Cable';
    default: return null;
  }
}

function routeInstructionKey(kind: WindowsInterceptionKind | null): string {
  return kind === 'voxveil-cable-relay'
    ? 'systemAudio.routeInstructionVoxveilCable'
    : 'systemAudio.routeInstructionVbCable';
}

export function SystemAudioEndpoints({
  endpoints,
  backendStatus,
  backendKind,
  physicalOutputs,
  selectedPhysicalOutputId,
  busy,
  installBusyId,
  error,
  onRefresh,
  onInstall,
  onInstallAll,
  onSelectPhysicalOutput,
  onOpenSoundSettings,
  onGetVbCable,
}: SystemAudioEndpointsProps) {
  const { t } = useTranslation();
  const installableCount = endpoints.filter((endpoint) => endpoint.status === 'installable').length;
  const label = backendLabel(backendKind);
  const missingVbCable = backendStatus === 'component-required' && backendKind !== 'apo';
  const usingRelay = backendKind === 'vb-cable-relay' || backendKind === 'voxveil-cable-relay';

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

      <div className="system-audio-row">
        <div className="system-audio-identity">
          <div className="system-audio-name">
            <strong>{t('systemAudio.routeTitle')}</strong>
            {label && <span className="endpoint-badge">{label}</span>}
          </div>
          {missingVbCable && <small>{t('systemAudio.vbCableMissing')}</small>}
          {usingRelay && backendStatus === 'routing-required' && <small>{t(routeInstructionKey(backendKind))}</small>}
          {usingRelay && backendStatus === 'ready' && <small>{t('systemAudio.routeActive')}</small>}
        </div>
        <div className="system-audio-state">
          {missingVbCable && (
            <button className="action-button" type="button" onClick={onGetVbCable}>
              {t('systemAudio.getVbCable')}
            </button>
          )}
          {usingRelay && backendStatus === 'routing-required' && (
            <button className="action-button" type="button" onClick={onOpenSoundSettings}>
              {t('systemAudio.openSoundSettings')}
            </button>
          )}
        </div>
      </div>

      {usingRelay && physicalOutputs.length > 0 && (
        <label className="system-audio-row">
          <span className="system-audio-identity">
            <strong>{t('systemAudio.physicalOutput')}</strong>
          </span>
          <select
            aria-label={t('systemAudio.physicalOutput')}
            value={selectedPhysicalOutputId ?? ''}
            onChange={(event) => onSelectPhysicalOutput(event.currentTarget.value)}
          >
            {!selectedPhysicalOutputId && <option value="">{t('systemAudio.selectPhysicalOutput')}</option>}
            {physicalOutputs.map((output) => (
              <option key={output.endpointId} value={output.endpointId}>
                {output.displayName}{output.isDefault ? ` (${t('systemAudio.default')})` : ''}
              </option>
            ))}
          </select>
        </label>
      )}

      {endpoints.length === 0 && !missingVbCable && !usingRelay && (
        <span className="meta">{t(busy ? 'systemAudio.detecting' : 'systemAudio.none')}</span>
      )}
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
