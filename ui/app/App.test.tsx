import { fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { App } from './App';

function sidebar() {
  return screen.getByRole('complementary', { name: 'Primary navigation' });
}

function open(name: string) {
  fireEvent.click(within(sidebar()).getByRole('button', { name }));
}

describe('App', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
  });

  it('renders local processing controls and updates vocal level', () => {
    render(<App />);
    expect(screen.getByLabelText('Voxveil')).toBeInTheDocument();
    const master = screen.getByRole('switch', { name: 'Processing' });
    expect(master).toHaveAttribute('aria-checked', 'true');
    fireEvent.click(master);
    expect(master).toHaveAttribute('aria-checked', 'false');

    fireEvent.change(screen.getByLabelText('Vocals'), { target: { value: '25' } });
    expect(screen.getByText('25%')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Classic DSP' }));
    expect(screen.getByRole('button', { name: 'Classic DSP' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('keeps communication sources bypassed and routes other apps', () => {
    render(<App />);
    open('Apps');
    const discord = screen.getByRole('switch', { name: 'Discord enabled' });
    expect(discord).toBeDisabled();
    const browser = screen.getByRole('switch', { name: 'Browser enabled' });
    fireEvent.click(browser);
    expect(browser).toHaveAttribute('aria-checked', 'false');
  });

  it('changes output routing and exposes engine licensing state', () => {
    render(<App />);
    open('Routing');
    fireEvent.click(screen.getByRole('button', { name: 'Virtual' }));
    expect(screen.getByRole('button', { name: 'Virtual' })).toHaveAttribute('aria-pressed', 'true');
    open('Engine');
    expect(screen.getAllByText('No model').length).toBeGreaterThan(0);
    expect(screen.getByText(/does not bundle AI weights/i)).toBeInTheDocument();
  });

  it('persists explicit theme choice and language locally', async () => {
    render(<App />);
    open('Settings');
    fireEvent.click(screen.getByRole('button', { name: 'Dark' }));
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(localStorage.getItem('voxveil.theme')).toBe('dark');

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'vi' } });
    expect(localStorage.getItem('voxveil.language')).toBe('vi');
    expect(await screen.findByRole('heading', { name: 'Cài đặt' })).toBeInTheDocument();
  });
});
