import { fireEvent, render, screen, within } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../../i18n';
import type { SystemAudioEndpoint } from '../../lib/types';
import { SystemAudioEndpoints } from './SystemAudioEndpoints';

const endpoint = (
  endpointId: string,
  displayName: string,
  status: SystemAudioEndpoint['status'],
  isDefault = false,
): SystemAudioEndpoint => ({ endpointId, displayName, adapterName: 'Example Audio', status, isDefault });

function renderPanel(endpoints: SystemAudioEndpoint[]) {
  const actions = {
    onRefresh: vi.fn(),
    onInstall: vi.fn(),
    onInstallAll: vi.fn(),
  };
  render(
    <I18nextProvider i18n={getI18n()}>
      <SystemAudioEndpoints
        endpoints={endpoints}
        busy={false}
        installBusyId={null}
        error={null}
        {...actions}
      />
    </I18nextProvider>,
  );
  return actions;
}

describe('SystemAudioEndpoints', () => {
  it('renders every discovered playback endpoint and marks the default', () => {
    renderPanel([
      endpoint('a', 'Speakers', 'installable', true),
      endpoint('b', 'USB DAC', 'ready'),
      endpoint('c', 'HDMI', 'unsupported'),
    ]);
    expect(screen.getByText('Speakers')).toBeInTheDocument();
    expect(screen.getByText('USB DAC')).toBeInTheDocument();
    expect(screen.getByText('HDMI')).toBeInTheDocument();
    expect(screen.getByText('Default')).toBeInTheDocument();
  });

  it('offers install only for safely installable endpoints', () => {
    const actions = renderPanel([
      endpoint('safe', 'Safe Speakers', 'installable'),
      endpoint('ambiguous', 'Ambiguous Speakers', 'ambiguous'),
      endpoint('unsupported', 'Unsupported HDMI', 'unsupported'),
      endpoint('unsigned', 'Resolved but unsigned', 'component-required'),
    ]);
    const safeRow = screen.getByTestId('system-audio-endpoint-safe');
    fireEvent.click(within(safeRow).getByRole('button', { name: 'Install' }));
    expect(actions.onInstall).toHaveBeenCalledWith('safe');
    expect(within(screen.getByTestId('system-audio-endpoint-ambiguous')).queryByRole('button', { name: 'Install' })).toBeNull();
    expect(within(screen.getByTestId('system-audio-endpoint-unsupported')).queryByRole('button', { name: 'Install' })).toBeNull();
    expect(within(screen.getByTestId('system-audio-endpoint-unsigned')).queryByRole('button', { name: 'Install' })).toBeNull();
  });

  it('shows bulk install only when at least two endpoints are installable', () => {
    const { rerender } = render(
      <I18nextProvider i18n={getI18n()}>
        <SystemAudioEndpoints
          endpoints={[endpoint('a', 'A', 'installable')]}
          busy={false}
          installBusyId={null}
          error={null}
          onRefresh={vi.fn()}
          onInstall={vi.fn()}
          onInstallAll={vi.fn()}
        />
      </I18nextProvider>,
    );
    expect(screen.queryByRole('button', { name: 'Install all compatible outputs' })).toBeNull();
    rerender(
      <I18nextProvider i18n={getI18n()}>
        <SystemAudioEndpoints
          endpoints={[endpoint('a', 'A', 'installable'), endpoint('b', 'B', 'installable')]}
          busy={false}
          installBusyId={null}
          error={null}
          onRefresh={vi.fn()}
          onInstall={vi.fn()}
          onInstallAll={vi.fn()}
        />
      </I18nextProvider>,
    );
    expect(screen.getByRole('button', { name: 'Install all compatible outputs' })).toBeInTheDocument();
  });
});
