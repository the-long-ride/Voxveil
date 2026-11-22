import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { I18nextProvider } from 'react-i18next';
import { getI18n } from '../i18n';
import type { SystemAudioEndpoint, VoxveilState } from '../lib/types';
import { nativeState } from './voxveilStateTestFixtures';
import { AppShell } from './AppShell';

const client = vi.hoisted(() => ({
  getState: vi.fn<() => Promise<VoxveilState>>(),
  listSystemAudioEndpoints: vi.fn<() => Promise<SystemAudioEndpoint[]>>(),
  listAudioOutputs: vi.fn(async () => []),
  setMasterEnabled: vi.fn(async () => undefined),
}));

vi.mock('../lib/tauri', () => ({ createVoxveilClient: () => client }));
vi.mock('../features/engine/useAiModelManager', () => ({
  useAiModelManager: () => ({ status: { installed: false, runtimeAvailable: false } }),
}));

afterEach(() => {
  cleanup();
  delete (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__;
  vi.resetAllMocks();
});

describe('header processing switch', () => {
  it.each(['routing-required', 'ready'] as const)('starts an automatic endpoint relay on the first click while endpoint enumeration is pending (%s)', async (backendStatus) => {
    (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
    client.getState.mockResolvedValue({
      ...nativeState,
      audioRouteChoice: 'legacy-automatic',
      backendKind: 'endpoint-relay',
      backendStatus,
      masterEnabled: false,
      physicalOutputEndpointId: 'line-1',
      physicalOutput: 'Line 1',
    });
    client.listSystemAudioEndpoints.mockReturnValue(new Promise(() => {}));
    client.listAudioOutputs.mockResolvedValue([]);
    client.setMasterEnabled.mockResolvedValue(undefined);
    render(
      <I18nextProvider i18n={getI18n()}>
        <AppShell themeMode="light" onThemeModeChange={vi.fn()} />
      </I18nextProvider>,
    );
    const toggle = screen.getByRole('switch', { name: 'Processing' });
    await waitFor(() => expect(toggle).toBeEnabled());
    expect(screen.getByRole('button', { name: 'Refreshing…' })).toBeDisabled();

    fireEvent.click(toggle);

    expect(client.setMasterEnabled).toHaveBeenCalledExactlyOnceWith(true);
    expect(toggle).toHaveAttribute('aria-checked', 'true');
  });
});
