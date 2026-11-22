import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { I18nextProvider } from 'react-i18next';
import { getI18n } from '../../i18n';
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

  it('resumes paused playback and reports player errors', () => {
    const callbacks = renderPlayer('paused');
    fireEvent.click(screen.getByRole('button', { name: 'Resume' }));
    expect(callbacks.onResume).toHaveBeenCalledOnce();
    expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled();
  });
});
