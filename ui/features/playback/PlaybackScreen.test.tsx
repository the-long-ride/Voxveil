import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { I18nextProvider } from 'react-i18next';
import { getI18n } from '../../i18n';
import { SUPPORTED_LANGUAGES } from '../../i18n/languages';
import type { AudioPlaybackSnapshot } from '../../lib/types';
import type { PlaybackScreenProps } from './PlaybackScreen';
import { PlaybackScreen } from './PlaybackScreen';

function playback(status: AudioPlaybackSnapshot['status']): AudioPlaybackSnapshot {
  return {
    status,
    fileName: 'song.flac',
    outputEndpointId: 'speakers',
    outputName: 'Speakers',
    sampleRate: 48_000,
    positionFrames: 24_000,
    totalFrames: 96_000,
    error: null,
  };
}

function renderPlayer(status: AudioPlaybackSnapshot['status'], overrides: Partial<PlaybackScreenProps> = {}) {
  const callbacks = {
    onOpen: vi.fn(),
    onPause: vi.fn(),
    onResume: vi.fn(),
    onStop: vi.fn(),
    onSeek: vi.fn(),
    onSelectOutput: vi.fn(),
  };
  render(
    <I18nextProvider i18n={getI18n()}>
      <PlaybackScreen
        playback={playback(status)}
        outputs={[{ endpointId: 'speakers', displayName: 'Speakers', isDefault: true }]}
        selectedOutputId="speakers"
        operationError={null}
        {...callbacks}
        {...overrides}
      />
    </I18nextProvider>,
  );
  return callbacks;
}

describe('PlaybackScreen', () => {
  it.each(SUPPORTED_LANGUAGES)('keeps the Player intro engine-neutral in %s', (language) => {
    expect(getI18n().getFixedT(language)('playback.description')).not.toMatch(/DSP/i);
  });
  it.each([
    [10450, 11], [1600, 2], [1800, 2], [2000, 2], [2001, 3], [0, 1],
  ])('rounds %i ms of AI preparation up to %i seconds', (preparingWaitMs, seconds) => {
    renderPlayer('buffering', { playback: { ...playback('buffering'), preparingWaitMs } });
    expect(screen.getByText(`Preparing AI… about ${seconds} s remaining`)).toBeInTheDocument();
  });
  it('shows AI buffering while keeping pause, stop and seek available', () => {
    renderPlayer('buffering');
    expect(screen.getByText('Preparing AI…')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Pause' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled();
    expect(screen.getByRole('slider')).toBeEnabled();
  });
  it('shows the selected output before any file has been opened', () => {
    renderPlayer('stopped', {
      playback: { ...playback('stopped'), fileName: null, outputName: null, outputEndpointId: null },
      outputs: [], selectedOutputId: 'line-1', selectedOutputName: 'Line 1',
    });
    expect(screen.getByText('Line 1')).toBeInTheDocument();
    expect(screen.queryByText('No output selected')).toBeNull();
  });

  it('can resolve the selected output name from the output list', () => {
    renderPlayer('stopped', { playback: { ...playback('stopped'), outputName: null } });
    expect(screen.getByText('Speakers', { selector: '.player-track span' })).toBeInTheDocument();
  });

  it('keeps the active playback output authoritative', () => {
    renderPlayer('playing', { selectedOutputName: 'Another output', selectedOutputId: 'other' });
    expect(screen.getByText('Speakers', { selector: '.player-track span' })).toBeInTheDocument();
  });

  it('shows no output when neither playback nor a selection has one', () => {
    renderPlayer('stopped', {
      playback: { ...playback('stopped'), outputName: null },
      outputs: [], selectedOutputId: null, selectedOutputName: 'System Default',
    });
    expect(screen.getByText('No output selected')).toBeInTheDocument();
  });

  it('exposes pause, stop, seek and physical output while a file is playing', () => {
    const callbacks = renderPlayer('playing');
    expect(screen.getByText('song.flac')).toBeInTheDocument();
    expect(screen.getByText(/only the file opened here/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }));
    fireEvent.change(screen.getByRole('slider', { name: 'Seek' }), { target: { value: '72' } });
    fireEvent.keyUp(screen.getByRole('slider', { name: 'Seek' }), { key: 'ArrowRight' });
    expect(callbacks.onPause).toHaveBeenCalledOnce();
    expect(callbacks.onStop).toHaveBeenCalledOnce();
    expect(callbacks.onSeek).toHaveBeenCalledWith(69_120);
  });

  it('does not seek on key or pointer release without a slider change', () => {
    const { onSeek } = renderPlayer('paused');
    const slider = screen.getByRole('slider', { name: 'Seek' });
    slider.focus();

    for (const key of ['Control', 'Shift', 'Alt', 'Meta', 'Tab', 'ArrowRight']) {
      fireEvent.keyUp(slider, { key });
    }
    fireEvent.pointerUp(slider);
    expect(onSeek).not.toHaveBeenCalled();
  });

  it('commits a keyboard change only on release of a slider key', () => {
    const { onSeek } = renderPlayer('paused');
    const slider = screen.getByRole('slider', { name: 'Seek' });
    slider.focus();

    fireEvent.change(slider, { target: { value: '0' } });
    fireEvent.keyUp(slider, { key: 'Control' });
    expect(onSeek).not.toHaveBeenCalled();
    expect(slider).toHaveValue('0');

    fireEvent.keyUp(slider, { key: 'Home' });
    expect(onSeek).toHaveBeenCalledExactlyOnceWith(0);
  });

  it('commits a changed pointer value once on release', () => {
    const { onSeek } = renderPlayer('paused');
    const slider = screen.getByRole('slider', { name: 'Seek' });

    fireEvent.pointerDown(slider);
    fireEvent.change(slider, { target: { value: '50' } });
    expect(onSeek).not.toHaveBeenCalled();
    fireEvent.pointerUp(slider);
    fireEvent.pointerUp(slider);
    expect(onSeek).toHaveBeenCalledExactlyOnceWith(48_000);
  });

  it('resumes paused playback and reports player errors', () => {
    const callbacks = renderPlayer('paused');
    fireEvent.click(screen.getByRole('button', { name: 'Resume' }));
    expect(callbacks.onResume).toHaveBeenCalledOnce();
    expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled();
  });

  it('lets users seek and stop after natural completion', () => {
    const callbacks = renderPlayer('ended', {
      playback: { ...playback('ended'), positionFrames: 96_000 },
    });
    const slider = screen.getByRole('slider', { name: 'Seek' });
    expect(slider).toBeEnabled();
    fireEvent.change(slider, { target: { value: '25' } });
    fireEvent.keyUp(slider, { key: 'ArrowLeft' });
    expect(callbacks.onSeek).toHaveBeenCalledWith(24_000);
    const stop = screen.getByRole('button', { name: 'Stop' });
    expect(stop).toBeEnabled();
    fireEvent.click(stop);
    expect(callbacks.onStop).toHaveBeenCalledOnce();
  });

  it('disables the seek control when a stopped file still has metadata', () => {
    renderPlayer('stopped');
    expect(screen.getByRole('slider', { name: 'Seek' })).toBeDisabled();
  });
});
