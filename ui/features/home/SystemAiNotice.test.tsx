import { render, screen } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it } from 'vitest';
import { getI18n } from '../../i18n';
import { SystemAiNotice } from './SystemAiNotice';

describe('system-wide AI relay status', () => {
  it('announces buffering, running and a deadline fallback', () => {
    const view = render(<I18nextProvider i18n={getI18n()}><SystemAiNotice status="buffering" /></I18nextProvider>);
    expect(screen.getByRole('status')).toHaveTextContent('Buffering AI (~12 s)');
    view.rerender(<I18nextProvider i18n={getI18n()}><SystemAiNotice status="running" /></I18nextProvider>);
    expect(screen.getByRole('status')).toHaveTextContent('AI relay active');
    view.rerender(<I18nextProvider i18n={getI18n()}><SystemAiNotice status="fell-behind" /></I18nextProvider>);
    expect(screen.getByRole('status')).toHaveTextContent('AI fell behind; using DSP');
  });
  it('hides the notice when opted out and explains missing models/runtime', () => {
    const view = render(<I18nextProvider i18n={getI18n()}><SystemAiNotice status="off" /></I18nextProvider>);
    expect(screen.queryByRole('status')).toBeNull();
    view.rerender(<I18nextProvider i18n={getI18n()}><SystemAiNotice status="unavailable" /></I18nextProvider>);
    expect(screen.getByRole('status')).toHaveTextContent('Spleeter or the app-owned runtime is unavailable');
  });
});
