import { render, screen } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../../i18n';
import type { VoxveilModel } from '../../app/useVoxveilState';
import { nativeState } from '../../app/voxveilStateTestFixtures';
import type { ProcessingBackendStatus, WindowsInterceptionKind } from '../../lib/types';
import { HomeScreen } from './HomeScreen';

function renderHome(backendKind: WindowsInterceptionKind, backendStatus: ProcessingBackendStatus, canStartProcessing: boolean, backendDetail = 'Relay is configured and ready to start', aiReady = false, fileRoute = false) {
  const model: VoxveilModel = {
    state: {
      ...nativeState, audioRouteChoice: fileRoute ? 'owned-file-playback' : 'legacy-automatic', masterEnabled: false,
      backendKind, backendStatus, backendDetail,
    },
    canStartProcessing,
    systemAudioEndpoints: [], physicalOutputs: [],
    systemAudioEndpointsBusy: false, systemAudioInstallBusyId: null, systemAudioInstallError: null,
    setVocalLevel: vi.fn(), setQuality: vi.fn(),
    setProcessingMode: vi.fn(), setEngine: vi.fn(), setAiPreferences: vi.fn(),
    refreshSystemAudioEndpoints: vi.fn(), installSystemAudioEndpoint: vi.fn(),
    selectPhysicalOutput: vi.fn(), openWindowsSoundSettings: vi.fn(), openVbCableDownload: vi.fn(),
    audioRouteError: null, processingOperationError: null, playbackOperationError: null,
    openAudioFile: vi.fn(async () => {}), pausePlayback: vi.fn(async () => {}),
    resumePlayback: vi.fn(async () => {}), seekPlayback: vi.fn(async () => {}), stopPlayback: vi.fn(async () => {}),
    setAudioRoute: vi.fn(async () => {}), setMasterEnabled: vi.fn(),
    setClassicSuppressionProfile: vi.fn(), setOutputMode: vi.fn(), setAppEnabled: vi.fn(),
  };
  render(<I18nextProvider i18n={getI18n()}><HomeScreen model={model} aiModelReady={aiReady} /></I18nextProvider>);
}

describe('Home readiness notice', () => {
  it('enables Auto and AI for file playback when the selected model and runtime are ready', () => {
    renderHome('endpoint-relay', 'ready', false, '', true, true);
    expect(screen.getByRole('button', { name: 'Auto' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'AI' })).toBeEnabled();
  });
  it('disables AI with an explanation while retaining Auto and DSP', () => {
    renderHome('endpoint-relay', 'ready', false, '', false, true);
    expect(screen.getByRole('button', { name: 'Auto' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Classic DSP' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'AI' })).toBeDisabled();
    expect(screen.getByText(/AI is unavailable.*selected model/)).toBeInTheDocument();
  });
  it.each(['endpoint-relay', 'vb-cable-relay', 'voxveil-cable-relay'] as const)('shows the configured %s detail without a readiness title', (kind) => {
    renderHome(kind, 'routing-required', true);
    expect(screen.queryByText('Ready')).toBeNull();
    expect(screen.getByText('Relay is configured and ready to start')).toBeInTheDocument();
    expect(screen.queryByText('Processing unavailable')).toBeNull();
  });

  it('shows the cable-not-default instruction without claiming readiness', () => {
    const instruction = 'Set CABLE Input as the Windows default output, then return to Voxveil';
    renderHome('vb-cable-relay', 'routing-required', true, instruction);
    expect(screen.getByText(instruction)).toBeInTheDocument();
    expect(screen.queryByText('Ready')).toBeNull();
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
