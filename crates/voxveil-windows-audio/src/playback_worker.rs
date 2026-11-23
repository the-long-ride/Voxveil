use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, TryRecvError},
};
use std::thread;
use std::time::Duration;

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::SpectralCenterSuppressor;
use voxveil_media::{MAX_READ_FRAMES, MediaDecoder};
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

use crate::device::EndpointDescriptor;

use super::playback_output::{PlaybackOutput, WasapiOutput};
use super::{
    MAX_QUEUE_FRAMES, OwnedPlaybackSnapshot, PLAYBACK_DECODE_FRAMES, PlaybackCommand,
    PlaybackStatus, set_playback_state,
};

pub(super) fn run_playback_worker(
    path: PathBuf,
    endpoint: EndpointDescriptor,
    initial_vocal_level: u8,
    initial_profile: ClassicSuppressionProfile,
    commands: Receiver<PlaybackCommand>,
    state: Arc<Mutex<OwnedPlaybackSnapshot>>,
) -> Result<(), String> {
    let decoder = MediaDecoder::open(&path).map_err(|error| error.to_string())?;
    let output = WasapiOutput::open(&endpoint, decoder.info().sample_rate)?;
    run_playback(
        decoder,
        output,
        initial_vocal_level,
        initial_profile,
        commands,
        state,
    )
}

fn run_playback(
    mut decoder: MediaDecoder,
    mut output: impl PlaybackOutput,
    initial_vocal_level: u8,
    initial_profile: ClassicSuppressionProfile,
    commands: Receiver<PlaybackCommand>,
    state: Arc<Mutex<OwnedPlaybackSnapshot>>,
) -> Result<(), String> {
    let info = decoder.info();
    let mut processor = new_processor(info.sample_rate, initial_vocal_level, initial_profile)?;
    let mut current_vocal_level = initial_vocal_level;
    let mut current_profile = initial_profile;
    let mut queue = VecDeque::<u8>::with_capacity(MAX_QUEUE_FRAMES * 8);
    let mut pcm = Vec::<f32>::with_capacity(PLAYBACK_DECODE_FRAMES * 2);
    let mut status = PlaybackStatus::Playing;
    let mut source_eof = false;
    let mut tail_flushed = false;

    output.start()?;
    set_playback_state(&state, |snapshot| {
        snapshot.status = PlaybackStatus::Playing;
        snapshot.sample_rate = Some(info.sample_rate);
        snapshot.total_frames = info.total_frames;
        snapshot.position_frames = 0;
        snapshot.error = None;
    });

    let result = (|| -> Result<(), String> {
        'playback: loop {
            loop {
                let command = if status == PlaybackStatus::Playing {
                    commands.try_recv()
                } else {
                    commands.recv().map_err(|_| TryRecvError::Disconnected)
                };
                match command {
                    Ok(PlaybackCommand::Stop) => break 'playback Ok(()),
                    Ok(PlaybackCommand::Pause) if status == PlaybackStatus::Playing => {
                        output.stop()?;
                        status = PlaybackStatus::Paused;
                        set_playback_state(&state, |snapshot| snapshot.status = status);
                    }
                    Ok(PlaybackCommand::Resume) if status == PlaybackStatus::Paused => {
                        output.start()?;
                        status = PlaybackStatus::Playing;
                        set_playback_state(&state, |snapshot| snapshot.status = status);
                    }
                    Ok(PlaybackCommand::Seek(frame)) => {
                        let total = info.total_frames.unwrap_or(frame);
                        let target = frame.min(total);
                        let resume_after_seek = status != PlaybackStatus::Paused;
                        if status == PlaybackStatus::Playing {
                            output.stop()?;
                            status = PlaybackStatus::Paused;
                        }
                        output.reset()?;
                        queue.clear();
                        processor =
                            new_processor(info.sample_rate, current_vocal_level, current_profile)?;
                        let actual = decoder.seek(target).map_err(|error| error.to_string())?;
                        source_eof = false;
                        tail_flushed = false;
                        if resume_after_seek {
                            output.start()?;
                            status = PlaybackStatus::Playing;
                        }
                        set_playback_state(&state, |snapshot| {
                            snapshot.position_frames = actual;
                            snapshot.status = status;
                            snapshot.error = None;
                        });
                    }
                    Ok(PlaybackCommand::SetVocalLevel(value)) => {
                        current_vocal_level = value.min(100);
                        let level = VocalLevel::new(current_vocal_level as f32 / 100.0)
                            .map_err(str::to_string)?;
                        processor.set_vocal_level(level);
                    }
                    Ok(PlaybackCommand::SetProfile(profile)) => {
                        current_profile = profile;
                        processor.set_profile(profile);
                    }
                    Ok(_) => {}
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break 'playback Ok(()),
                }
            }

            let queued_frames = queue.len() / 8;
            if !source_eof && queued_frames < MAX_QUEUE_FRAMES - PLAYBACK_DECODE_FRAMES {
                let frames = decoder
                    .read_frames(PLAYBACK_DECODE_FRAMES.min(MAX_READ_FRAMES), &mut pcm)
                    .map_err(|error| error.to_string())?;
                if frames > 0 {
                    processor.process_stereo_interleaved(&mut pcm);
                    enqueue_processed(&mut queue, &pcm)?;
                } else if !tail_flushed {
                    let tail_frames = processor.latency_frames();
                    pcm.clear();
                    pcm.resize(tail_frames.saturating_mul(2), 0.0);
                    processor.process_stereo_interleaved(&mut pcm);
                    enqueue_processed(&mut queue, &pcm)?;
                    tail_flushed = true;
                    source_eof = true;
                } else {
                    source_eof = true;
                }
            }

            let device_pending = output.render(&mut queue, source_eof)?;
            let queued_after_write = (queue.len() / 8) as u64;
            let audible_position = decoder
                .position_frames()
                .saturating_sub(queued_after_write)
                .saturating_sub(device_pending)
                .saturating_sub(processor.latency_frames() as u64);
            set_playback_state(&state, |snapshot| {
                snapshot.position_frames = info
                    .total_frames
                    .map_or(audible_position, |total| audible_position.min(total));
            });

            if source_eof && queue.is_empty() && device_pending == 0 {
                output.stop()?;
                status = PlaybackStatus::Ended;
                set_playback_state(&state, |snapshot| {
                    snapshot.status = status;
                    snapshot.position_frames =
                        info.total_frames.unwrap_or(decoder.position_frames());
                });
            } else {
                thread::sleep(Duration::from_millis(2));
            }
        }
    })();

    let stop_result = if status == PlaybackStatus::Playing {
        output.stop()
    } else {
        Ok(())
    };
    match (result, stop_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn new_processor(
    sample_rate: u32,
    vocal_level: u8,
    profile: ClassicSuppressionProfile,
) -> Result<SpectralCenterSuppressor, String> {
    let level = VocalLevel::new(vocal_level.min(100) as f32 / 100.0).map_err(str::to_string)?;
    Ok(SpectralCenterSuppressor::new(sample_rate, level, profile))
}

pub(super) fn enqueue_processed(queue: &mut VecDeque<u8>, pcm: &[f32]) -> Result<(), String> {
    if pcm.len() % 2 != 0 {
        return Err("processed audio buffer ended between stereo frames".into());
    }
    let bytes = pcm
        .len()
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or_else(|| "processed audio buffer size overflow".to_string())?;
    let max_bytes = MAX_QUEUE_FRAMES * 8;
    if queue
        .len()
        .checked_add(bytes)
        .is_none_or(|total| total > max_bytes)
    {
        return Err("local playback exceeded the bounded output buffer".into());
    }
    for sample in pcm {
        queue.extend(sample.to_le_bytes());
    }
    Ok(())
}

#[cfg(test)]
#[path = "playback_worker_tests.rs"]
mod tests;
