import { fireEvent, render, screen } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../../i18n';
import { SettingsScreen } from './SettingsScreen';

describe('SettingsScreen', () => {
  it('lets users select automatic system-wide processing with a driverless fallback', () => {
    const onAudioRouteChange = vi.fn();
    render(
      <I18nextProvider i18n={getI18n()}>
        <SettingsScreen
          edition="standard"
          windowsAudioRoutesAvailable
          audioRouteChoice="physical-apo"
          audioRouteError={null}
          onAudioRouteChange={onAudioRouteChange}
          themeMode="dark"
          onThemeModeChange={vi.fn()}
        />
      </I18nextProvider>,
    );

    const route = screen.getByRole('combobox', { name: 'Audio processing route' });
    expect(screen.getByRole('radio', { name: 'Use UVR MDX Inst HQ3' })).toBeChecked();
    expect(screen.getByRole('option', { name: /automatic.*driverless fallback/i })).toBeInTheDocument();
    fireEvent.change(route, { target: { value: 'legacy-automatic' } });
    expect(onAudioRouteChange).toHaveBeenCalledWith('legacy-automatic');
  });

  it('describes the automatic route order and labels Appearance once', () => {
    render(
      <I18nextProvider i18n={getI18n()}>
        <SettingsScreen
          windowsAudioRoutesAvailable
          audioRouteChoice="legacy-automatic"
          audioRouteError={null}
          onAudioRouteChange={vi.fn()}
          themeMode="dark"
          onThemeModeChange={vi.fn()}
        />
      </I18nextProvider>,
    );

    expect(screen.getByRole('combobox', { name: 'Audio processing route' })).toHaveAccessibleDescription(
      /Choose where Voxveil applies Classic DSP\..*Voxveil APO.*default Windows output.*Voxveil Virtual Cable, then VB-CABLE.*driverless relay.*spare physical output.*different physical output/i,
    );
    expect(screen.getAllByText('Appearance')).toHaveLength(1);
    expect(screen.getByRole('group', { name: 'Appearance' })).toBeInTheDocument();
  });
});
