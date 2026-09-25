import { fireEvent, render, screen } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../../i18n';
import { SettingsScreen } from './SettingsScreen';

describe('SettingsScreen', () => {
  it('lets users select automatic system-wide processing with an existing VB-CABLE setup', () => {
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

    const route = screen.getByRole('combobox', { name: /processing route/i });
    expect(screen.getByRole('option', { name: /automatic.*APO or VB-CABLE/i })).toBeInTheDocument();
    fireEvent.change(route, { target: { value: 'legacy-automatic' } });
    expect(onAudioRouteChange).toHaveBeenCalledWith('legacy-automatic');
  });
});
