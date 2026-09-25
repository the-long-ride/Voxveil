import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ScreenIntro } from '../../components/ScreenIntro';
import type { AudioOutput, AudioPlaybackSnapshot } from '../../lib/types';

export interface PlaybackScreenProps {
  playback: AudioPlaybackSnapshot;
  outputs: AudioOutput[];
  selectedOutputId: string | null;
  operationError: string | null;
  onOpen: () => void | Promise<void>;
  onPause: () => void | Promise<void>;
  onResume: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onSeek: (positionFrames: number) => void | Promise<void>;
  onSelectOutput: (endpointId: string) => void;
}

function clockTime(frames: number, sampleRate: number | null): string {
  if (!sampleRate || !Number.isFinite(frames)) return '0:00';
  const seconds = Math.max(0, Math.floor(frames / sampleRate));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

export function PlaybackScreen({
  playback,
  outputs,
  selectedOutputId,
  operationError,
  onOpen,
  onPause,
  onResume,
  onStop,
  onSeek,
  onSelectOutput,
}: PlaybackScreenProps) {
  const { t } = useTranslation();
  const [seekDraft, setSeekDraft] = useState<number | null>(null);
  const { status, totalFrames, positionFrames, sampleRate } = playback;
  const duration = totalFrames ?? 0;
  const seekPercent = duration > 0 ? Math.min(100, (positionFrames / duration) * 100) : 0;
  const sliderValue = seekDraft ?? seekPercent;
  const canSeek = duration > 0 && sampleRate !== null && status !== 'loading';
  const isActive = status === 'playing' || status === 'paused';
  const error = operationError ?? playback.error;
  const commitSeek = (value: number) => {
    setSeekDraft(null);
    if (canSeek) void onSeek(Math.round((Math.min(100, Math.max(0, value)) / 100) * duration));
  };

  return (
    <section className="screen owned-player" aria-labelledby="playback-title">
      <ScreenIntro
        id="playback-title"
        title={t('playback.title')}
        description={t('playback.description')}
      />

      <div className="player-track" aria-live="polite">
        <span className="meta">{t(`playback.status.${status}`)}</span>
        <strong>{playback.fileName ?? t('playback.noFile')}</strong>
        <span>{playback.outputName ?? t('playback.noOutput')}</span>
      </div>

      <div className="player-controls" aria-label={t('playback.controls')}>
        <button className="action-button" type="button" onClick={() => void onOpen()} disabled={status === 'loading'}>
          {t('playback.open')}
        </button>
        {status === 'playing' && (
          <button className="action-button is-subtle" type="button" onClick={() => void onPause()}>
            {t('playback.pause')}
          </button>
        )}
        {status === 'paused' && (
          <button className="action-button is-subtle" type="button" onClick={() => void onResume()}>
            {t('playback.resume')}
          </button>
        )}
        <button className="action-button is-subtle" type="button" onClick={() => void onStop()} disabled={!isActive}>
          {t('playback.stop')}
        </button>
      </div>

      <div className="settings-row player-output">
        <span>
          <strong>{t('playback.output')}</strong>
          <small>{t('playback.outputDescription')}</small>
        </span>
        <select
          aria-label={t('playback.output')}
          value={selectedOutputId ?? ''}
          onChange={(event) => onSelectOutput(event.currentTarget.value)}
        >
          <option value="" disabled>{t('playback.chooseOutput')}</option>
          {outputs.map((output) => (
            <option key={output.endpointId} value={output.endpointId}>
              {output.displayName}{output.isDefault ? ` (${t('systemAudio.default')})` : ''}
            </option>
          ))}
        </select>
      </div>

      <div className="range-control player-seek">
        <label htmlFor="player-seek">{t('playback.seek')}</label>
        <input
          id="player-seek"
          type="range"
          min="0"
          max="100"
          step="0.1"
          value={sliderValue}
          disabled={!canSeek}
          onChange={(event) => setSeekDraft(Number(event.currentTarget.value))}
          onPointerUp={(event) => commitSeek(Number(event.currentTarget.value))}
          onKeyUp={(event) => commitSeek(Number(event.currentTarget.value))}
        />
        <div className="range-labels mono">
          <span>{clockTime(positionFrames, sampleRate)}</span>
          <span>{clockTime(duration, sampleRate)}</span>
        </div>
      </div>

      <p className="privacy-note">{t('playback.onlyFile')}</p>
      {error && <p className="model-error" role="alert">{error}</p>}
    </section>
  );
}
