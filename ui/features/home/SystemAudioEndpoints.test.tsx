import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../../i18n';
import type { AudioOutput, AudioRouteChoice, ProcessingBackendStatus, SystemAudioEndpoint, WindowsInterceptionKind } from '../../lib/types';
import { SystemAudioEndpoints } from './SystemAudioEndpoints';

const endpoint = (
  endpointId: string,
  displayName: string,
  status: SystemAudioEndpoint['status'],
  isDefault = false,
): SystemAudioEndpoint => ({ endpointId, displayName, adapterName: 'Example Audio', status, isDefault });

const output = (endpointId: string, displayName: string, isDefault = false): AudioOutput => ({
  endpointId,
  displayName,
  isDefault,
});

function renderPanel(
  endpoints: SystemAudioEndpoint[],
  options: {
    backendStatus?: ProcessingBackendStatus;
    backendKind?: WindowsInterceptionKind | null;
    audioRouteChoice?: AudioRouteChoice;
    busy?: boolean;
    physicalOutputs?: AudioOutput[];
    selectedPhysicalOutputId?: string | null;
    onSelectPhysicalOutput?: (endpointId: string) => void | Promise<void>;
  } = {},
) {
  const actions = {
    onRefresh: vi.fn(),
    onInstall: vi.fn(),
    onSelectPhysicalOutput: options.onSelectPhysicalOutput ?? vi.fn(),
    onOpenSoundSettings: vi.fn(),
    onGetVbCable: vi.fn(),
  };
  render(
    <I18nextProvider i18n={getI18n()}>
      <SystemAudioEndpoints
        endpoints={endpoints}
        backendStatus={options.backendStatus ?? 'component-required'}
        backendKind={options.backendKind ?? null}
        audioRouteChoice={options.audioRouteChoice ?? 'physical-apo'}
        physicalOutputs={options.physicalOutputs ?? []}
        selectedPhysicalOutputId={options.selectedPhysicalOutputId ?? null}
        busy={options.busy ?? false}
        installBusyId={null}
        error={null}
        {...actions}
      />
    </I18nextProvider>,
  );
  return actions;
}

describe('SystemAudioEndpoints', () => {
  it('offers the official VB-CABLE action when the relay component is missing', () => {
    const actions = renderPanel([], { backendStatus: 'component-required', audioRouteChoice: 'legacy-automatic' });
    fireEvent.click(screen.getByRole('button', { name: 'Get VB-CABLE' }));
    expect(actions.onGetVbCable).toHaveBeenCalledOnce();
  });

  it('does not offer VB-CABLE installation for the physical APO route', () => {
    renderPanel([], { backendStatus: 'component-required', audioRouteChoice: 'physical-apo' });

    expect(screen.queryByRole('button', { name: 'Get VB-CABLE' })).toBeNull();
    expect(screen.getByText(/physical APO route is selected.*installed VB-CABLE/i)).toBeInTheDocument();
  });

  it('clarifies that an endpoint unavailable for Voxveil APO installation may still be a playback device', () => {
    renderPanel([{
      ...endpoint('cable-input', 'CABLE Input', 'unsupported'),
      detail: 'No unambiguous topology AddInterface reference was found in the installed audio driver INF.',
    }]);

    expect(screen.getByText('Voxveil APO not installable')).toBeInTheDocument();
    expect(screen.getByText(/No unambiguous topology AddInterface reference/)).toBeInTheDocument();
  });

  it('guides VB-CABLE routing and allows a physical sink selection', () => {
    const actions = renderPanel([], {
      backendStatus: 'routing-required',
      backendKind: 'vb-cable-relay',
      physicalOutputs: [output('speakers', 'Speakers'), output('dac', 'USB DAC')],
      selectedPhysicalOutputId: 'speakers',
    });
    expect(screen.getByText(/Set CABLE Input as the Windows output/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Open Sound settings' }));
    fireEvent.change(screen.getByRole('combobox', { name: 'Physical output' }), {
      target: { value: 'dac' },
    });
    expect(actions.onOpenSoundSettings).toHaveBeenCalledOnce();
    expect(actions.onSelectPhysicalOutput).toHaveBeenCalledWith('dac');
  });

  it('disables physical output changes while endpoint refresh is running', () => {
    renderPanel([], {
      backendStatus: 'routing-required',
      backendKind: 'vb-cable-relay',
      audioRouteChoice: 'legacy-automatic',
      physicalOutputs: [output('speakers', 'Speakers')],
      selectedPhysicalOutputId: 'speakers',
      busy: true,
    });

    expect(screen.getByRole('combobox', { name: 'Physical output' })).toBeDisabled();
  });

  it('disables refresh and output selection until a physical output change finishes', async () => {
    let completeSelection: (() => void) | undefined;
    const onSelectPhysicalOutput = vi.fn(() => new Promise<void>((resolve) => { completeSelection = resolve; }));
    renderPanel([], {
      backendStatus: 'routing-required',
      backendKind: 'vb-cable-relay',
      audioRouteChoice: 'legacy-automatic',
      physicalOutputs: [output('speakers', 'Speakers'), output('dac', 'USB DAC')],
      selectedPhysicalOutputId: 'speakers',
      onSelectPhysicalOutput,
    });

    fireEvent.change(screen.getByRole('combobox', { name: 'Physical output' }), { target: { value: 'dac' } });

    expect(screen.getByRole('combobox', { name: 'Physical output' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Refresh' })).toBeDisabled();
    completeSelection?.();
    await waitFor(() => expect(screen.getByRole('combobox', { name: 'Physical output' })).toBeEnabled());
    expect(screen.getByRole('combobox', { name: 'Physical output' })).toBeEnabled();
  });

  it('uses Voxveil Input guidance for the first-party relay', () => {
    renderPanel([], {
      backendStatus: 'routing-required',
      backendKind: 'voxveil-cable-relay',
      physicalOutputs: [output('speakers', 'Speakers')],
      selectedPhysicalOutputId: 'speakers',
    });
    expect(screen.getByText(/Set Voxveil Input as the Windows output/)).toBeInTheDocument();
    expect(screen.queryByText(/Set CABLE Input as the Windows output/)).toBeNull();
  });

  it('shows the active VB-CABLE route when ready', () => {
    renderPanel([], {
      backendStatus: 'ready',
      backendKind: 'vb-cable-relay',
      physicalOutputs: [output('speakers', 'Speakers')],
      selectedPhysicalOutputId: 'speakers',
    });
    expect(screen.getByText('VB-CABLE')).toBeInTheDocument();
    expect(screen.getByText('Speakers')).toBeInTheDocument();
  });

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

  it('keeps multiple installable outputs as explicit per-endpoint installs', () => {
    const actions = renderPanel([
      endpoint('a', 'A', 'installable'),
      endpoint('b', 'B', 'installable'),
    ]);

    expect(screen.queryByRole('button', { name: 'Install all compatible outputs' })).toBeNull();
    const first = screen.getByTestId('system-audio-endpoint-a');
    const second = screen.getByTestId('system-audio-endpoint-b');
    fireEvent.click(within(first).getByRole('button', { name: 'Install' }));
    fireEvent.click(within(second).getByRole('button', { name: 'Install' }));
    expect(actions.onInstall).toHaveBeenNthCalledWith(1, 'a');
    expect(actions.onInstall).toHaveBeenNthCalledWith(2, 'b');
  });

});
