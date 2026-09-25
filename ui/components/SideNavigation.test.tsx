import { fireEvent, render, screen } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { describe, expect, it, vi } from 'vitest';
import { getI18n } from '../i18n';
import { BottomNavigation } from './BottomNavigation';
import { SideNavigation } from './SideNavigation';

describe('SideNavigation', () => {
  it('lets a user leave Player and open Settings in local-file playback mode', () => {
    const onNavigate = vi.fn();
    render(
      <I18nextProvider i18n={getI18n()}>
        <SideNavigation active="playback" audioRouteChoice="owned-file-playback" onNavigate={onNavigate} />
      </I18nextProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Settings' }));

    expect(onNavigate).toHaveBeenCalledWith('settings');
  });

  it('keeps Settings reachable from the compact Player navigation', () => {
    const onNavigate = vi.fn();
    render(
      <I18nextProvider i18n={getI18n()}>
        <BottomNavigation active="playback" audioRouteChoice="owned-file-playback" onNavigate={onNavigate} />
      </I18nextProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Settings' }));

    expect(onNavigate).toHaveBeenCalledWith('settings');
  });
});
