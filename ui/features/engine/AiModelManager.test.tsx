import { fireEvent, render, screen } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../../i18n';
import type { AiModelController } from './useAiModelManager';
import { AiModelManager } from './AiModelManager';

function controller(overrides: Partial<AiModelController> = {}): AiModelController {
  return {
    status: {
      id: 'htdemucs-ft-vocals-fp16', displayName: 'HT-Demucs FT Vocals FP16', approximateSizeMb: 166,
      license: 'MIT', source: 'StemSplitio/htdemucs-ft-vocals-onnx', sourceRevision: 'abc',
      installed: false, runtimeAvailable: false, bundled: false, downloadAvailable: true, consentRequired: true,
    },
    native: true, busy: false, progress: null, error: null,
    install: vi.fn(async () => undefined), remove: vi.fn(async () => undefined),
    ...overrides,
  };
}

function renderManager(value: AiModelController) {
  return render(<I18nextProvider i18n={getI18n()}><AiModelManager controller={value} /></I18nextProvider>);
}

describe('AiModelManager', () => {
  it('requires explicit consent before download', () => {
    const value = controller();
    renderManager(value);
    fireEvent.click(screen.getByRole('button', { name: 'Install AI model' }));
    const download = screen.getByRole('button', { name: 'Agree & download' });
    expect(download).toBeDisabled();
    fireEvent.click(screen.getByRole('checkbox'));
    expect(download).toBeEnabled();
    fireEvent.click(download);
    expect(value.install).toHaveBeenCalledWith(true);
  });

  it('offers removal after the model is installed', () => {
    const value = controller({ status: { ...controller().status, installed: true } });
    renderManager(value);
    fireEvent.click(screen.getByRole('button', { name: 'Remove model' }));
    expect(value.remove).toHaveBeenCalledTimes(1);
  });

  it('shows bundled installation and hides download and removal for package resources', () => {
    const value = controller({ status: { ...controller().status, installed: true, bundled: true, runtimeAvailable: true } });
    renderManager(value);
    expect(screen.getByText('Bundled · installed')).toBeInTheDocument();
    expect(screen.getByText(/part of this application.*cannot be removed/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Remove model' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Install AI model' })).not.toBeInTheDocument();
    expect(value.remove).not.toHaveBeenCalled();
  });
});
