import { fireEvent, render, screen } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../../i18n';
import { AiModelsSettings } from './AiModelsSettings';

describe('AI model settings', () => {
  it('shows both models, sizes and UVR credit and persists the selected model', () => {
    const onChange = vi.fn();
    render(<I18nextProvider i18n={getI18n()}><AiModelsSettings preferences={{ model: 'spleeter-2stems-fp16', systemWide: true }} onChange={onChange} /></I18nextProvider>);
    expect(screen.getByRole('radio', { name: 'Use Spleeter 2-stems FP16' })).toBeChecked();
    expect(screen.getByText(/~40 MB/)).toBeInTheDocument();
    expect(screen.getByText(/~67 MB/)).toBeInTheDocument();
    expect(screen.getByText(/Ultimate Vocal Remover.*KUIELab/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('radio', { name: 'Use UVR MDX Inst HQ3' }));
    expect(onChange).toHaveBeenCalledWith({ model: 'mdx-inst-hq3', systemWide: false });
  });
  it('warns about the delay and only allows the opt-in for Spleeter', () => {
    const onChange = vi.fn();
    const view = render(<I18nextProvider i18n={getI18n()}><AiModelsSettings preferences={{ model: 'spleeter-2stems-fp16', systemWide: false }} onChange={onChange} /></I18nextProvider>);
    expect(screen.getByText(/12 seconds.*video and games/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('switch', { name: 'Use AI for system-wide audio' }));
    expect(onChange).toHaveBeenCalledWith({ model: 'spleeter-2stems-fp16', systemWide: true });
    view.rerender(<I18nextProvider i18n={getI18n()}><AiModelsSettings preferences={{ model: 'mdx-inst-hq3', systemWide: false }} onChange={onChange} /></I18nextProvider>);
    expect(screen.getByRole('switch', { name: 'Use AI for system-wide audio' })).toBeDisabled();
    expect(screen.getByText(/MDX cannot keep up/)).toBeInTheDocument();
  });
});
