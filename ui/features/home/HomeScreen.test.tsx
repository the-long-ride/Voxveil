import { render, screen } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../../i18n';
import type { VoxveilModel } from '../../app/useVoxveilState';
import { nativeState } from '../../app/voxveilStateTestFixtures';
import type { ProcessingBackendStatus, WindowsInterceptionKind } from '../../lib/types';
import { HomeScreen } from './HomeScreen';

function renderHome(backendKind: WindowsInterceptionKind, backendStatus: ProcessingBackendStatus, canStartProcessing: boolean) {
  const model: VoxveilModel = {
    state: {
      ...nativeState, audioRouteChoice: 'legacy-automatic', masterEnabled: false,
      backendKind, backendStatus, backendDetail: 'Relay is configured and ready to start',
    },
    canStartProcessing,
    systemAudioEndpoints: [], physicalOutputs: [],
    systemAudioEndpointsBusy: false, systemAudioInstallBusyId: null, systemAudioInstallError: null,
    setVocalLevel: vi.fn(), setQuality: vi.fn(),
    setProcessingMode: vi.fn(), setEngine: vi.fn(),
    refreshSystemAudioEndpoints: vi.fn(), installSystemAudioEndpoint: vi.fn(),
    selectPhysicalOutput: vi.fn(), openWindowsSoundSettings: vi.fn(), openVbCableDownload: vi.fn(),
    audioRouteError: null, processingOperationError: null, playbackOperationError: null,
    openAudioFile: vi.fn(async () => {}), pausePlayback: vi.fn(async () => {}),
    resumePlayback: vi.fn(async () => {}), seekPlayback: vi.fn(async () => {}), stopPlayback: vi.fn(async () => {}),
    setAudioRoute: vi.fn(async () => {}), setMasterEnabled: vi.fn(),
    setClassicSuppressionProfile: vi.fn(), setOutputMode: vi.fn(), setAppEnabled: vi.fn(),
  };
  render(<I18nextProvider i18n={getI18n()}><HomeScreen model={model} aiModelReady={false} /></I18nextProvider>);
}

describe('Home readiness notice', () => {
  it.each(['endpoint-relay', 'vb-cable-relay', 'voxveil-cable-relay'] as const)('shows a configured %s as ready to start, not unavailable', (kind) => {
    renderHome(kind, 'routing-required', true);
    expect(screen.getByText('Ready')).toBeInTheDocument();
    expect(screen.getByText('Relay is configured and ready to start')).toBeInTheDocument();
    expect(screen.queryByText('Processing unavailable')).toBeNull();
  });

  it('still marks a relay with missing routing prerequisites as unavailable', () => {
    renderHome('endpoint-relay', 'routing-required', false);
    expect(screen.getByText('Processing unavailable')).toBeInTheDocument();
  });

  it('does not show an unavailable notice for a running endpoint relay', () => {
    renderHome('endpoint-relay', 'ready', true);
    expect(screen.queryByText('Processing unavailable')).toBeNull();
  });

  it('keeps relay faults visible', () => {
    renderHome('endpoint-relay', 'faulted', false);
    expect(screen.getByText('Processing unavailable')).toBeInTheDocument();
  });
});
