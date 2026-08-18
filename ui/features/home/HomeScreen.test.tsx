import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import type { VoxveilModel } from '../../app/useVoxveilState';
import { getI18n } from '../../i18n';
import { SAFE_NATIVE_STATE } from '../../lib/demo-state';
import { HomeScreen } from './HomeScreen';

function modelWithInstallFailure(error: unknown): VoxveilModel {
  return {
    state: { ...SAFE_NATIVE_STATE, backendStatus: 'component-required' },
    setMasterEnabled: vi.fn(),
    setProcessingMode: vi.fn(),
    setEngine: vi.fn(),
    setVocalLevel: vi.fn(),
    setQuality: vi.fn(),
    setOutputMode: vi.fn(),
    setAppEnabled: vi.fn(),
    installSystemAudioComponent: vi.fn().mockRejectedValue(error),
  } as unknown as VoxveilModel;
}

describe('HomeScreen system audio installation', () => {
  it('keeps diagnostics visible and opens details automatically when installation fails', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    const model = modelWithInstallFailure({
      message: 'Windows system-audio installation failed',
      exitCode: 1,
      stdout: 'Preparing Voxveil APO',
      stderr: 'Access to the registry key is denied.',
    });

    render(
      <I18nextProvider i18n={getI18n()}>
        <HomeScreen model={model} aiModelReady={false} />
      </I18nextProvider>,
    );

    const detailsButton = screen.getByRole('button', { name: 'View details' });
    expect(detailsButton).toBeVisible();
    expect(detailsButton).toBeDisabled();

    fireEvent.click(screen.getByRole('button', { name: 'Install system audio component' }));

    const dialog = await screen.findByRole('dialog', { name: 'System audio installation failed' });
    expect(screen.getByRole('button', { name: 'View details' })).toBeEnabled();
    expect(within(dialog).getByText(/Exit code: 1/)).toBeInTheDocument();
    expect(within(dialog).getByText(/Preparing Voxveil APO/)).toBeInTheDocument();
    expect(within(dialog).getByText(/Access to the registry key is denied/)).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole('button', { name: 'Copy details' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledOnce());
    expect(writeText.mock.calls[0]?.[0]).toContain('Access to the registry key is denied.');

    fireEvent.click(within(dialog).getByRole('button', { name: 'Close' }));
    expect(screen.queryByRole('dialog', { name: 'System audio installation failed' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'View details' }));
    expect(screen.getByRole('dialog', { name: 'System audio installation failed' })).toBeInTheDocument();
  });
});
