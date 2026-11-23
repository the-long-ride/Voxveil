use super::playback_output::{PlaybackOutput, WasapiOutput};
use super::{OwnedPlaybackSnapshot, PlaybackCommand, PlaybackStatus, set_playback_state};
use crate::device::EndpointDescriptor;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, TryRecvError},
    },
    thread,
    time::Duration,
};
use voxveil_ai::player::{AiPlaybackConfig, Prepared, StemProducer, mix_sample};
use voxveil_media::MediaDecoder;

pub(super) fn run(
    path: PathBuf,
    endpoint: EndpointDescriptor,
    mut level: u8,
    config: AiPlaybackConfig,
    commands: Receiver<PlaybackCommand>,
    state: Arc<Mutex<OwnedPlaybackSnapshot>>,
) -> Result<(), String> {
    let info = MediaDecoder::open(&path).map_err(|e| e.to_string())?.info();
    let mut output = WasapiOutput::open(&endpoint, info.sample_rate)?;
    let mut producer = Some(StemProducer::spawn(path.clone(), 0, config.clone()));
    let mut stems = VecDeque::<[f32; 2]>::new();
    let mut position = 0;
    let mut submitted = 0;
    let mut eof = false;
    let mut running = false;
    let mut paused = false;
    set_playback_state(&state, |s| {
        s.status = PlaybackStatus::Buffering;
        s.sample_rate = Some(info.sample_rate);
        s.total_frames = info.total_frames;
        s.error = None;
        s.preparing_wait_ms = None;
    });
    let result = (|| -> Result<(), String> {
        loop {
            loop {
                let command = match commands.try_recv() {
                    Ok(command) => command,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return Ok(()),
                };
                match command {
                    PlaybackCommand::Stop => return Ok(()),
                    PlaybackCommand::Pause => {
                        paused = true;
                        if running {
                            output.stop()?;
                            running = false;
                        }
                        if let Some(p) = &producer {
                            p.pause(true);
                        }
                        set_playback_state(&state, |s| s.status = PlaybackStatus::Paused);
                    }
                    PlaybackCommand::Resume => {
                        paused = false;
                        if let Some(p) = &producer {
                            p.pause(false);
                        }
                    }
                    PlaybackCommand::Seek(frame) => {
                        if running {
                            output.stop()?;
                            running = false;
                        }
                        output.reset()?;
                        set_playback_state(&state, |s| {
                            s.preparing_wait_ms = None;
                            s.status = if paused {
                                PlaybackStatus::Paused
                            } else {
                                PlaybackStatus::Buffering
                            };
                        });
                        producer.take();
                        position = info.total_frames.map_or(frame, |total| frame.min(total));
                        submitted = 0;
                        stems.clear();
                        eof = false;
                        let next = StemProducer::spawn(path.clone(), position, config.clone());
                        next.pause(paused);
                        producer = Some(next);
                        set_playback_state(&state, |s| s.position_frames = position);
                    }
                    PlaybackCommand::SetVocalLevel(value) => level = value,
                    PlaybackCommand::SetProfile(_) => {}
                }
            }
            if !paused {
                while stems.len() < 16384 {
                    let Some(p) = producer.as_mut() else {
                        break;
                    };
                    match p.poll()? {
                        Some(Prepared::Position(actual)) => position = actual,
                        Some(Prepared::Stems {
                            vocals,
                            accompaniment,
                        }) => {
                            stems
                                .extend(vocals.into_iter().zip(accompaniment).map(|(v, a)| [v, a]));
                        }
                        Some(Prepared::End) => {
                            eof = true;
                            producer.take();
                            break;
                        }
                        None => break,
                    }
                }
                if !stems.is_empty() {
                    // Keep unsubmitted audio as stems so slider edits affect it immediately.
                    let mut mixed: VecDeque<u8> = stems
                        .iter()
                        .flat_map(|[v, a]| mix_sample(*v, *a, level).to_le_bytes())
                        .collect();
                    let before = mixed.len();
                    let pending = output.render(&mut mixed, true)?;
                    let consumed = (before - mixed.len()) / 4;
                    stems.drain(..consumed);
                    submitted += consumed as u64 / 2;
                    if !running {
                        output.start()?;
                        running = true;
                    }
                    set_playback_state(&state, |s| {
                        s.status = PlaybackStatus::Playing;
                        s.preparing_wait_ms = None;
                        s.position_frames = position + submitted.saturating_sub(pending);
                    });
                } else if eof {
                    let pending = output.render(&mut VecDeque::new(), true)?;
                    if pending == 0 {
                        if running {
                            output.stop()?;
                            running = false;
                        }
                        set_playback_state(&state, |s| {
                            s.status = PlaybackStatus::Ended;
                            s.position_frames = info.total_frames.unwrap_or(position + submitted);
                        });
                    } else {
                        if !running {
                            output.start()?;
                            running = true;
                        }
                        set_playback_state(&state, |s| {
                            s.status = PlaybackStatus::Playing;
                            s.position_frames = position + submitted.saturating_sub(pending);
                        });
                    }
                } else {
                    if running {
                        output.stop()?;
                        running = false;
                    }
                    let wait_ms = producer.as_ref().and_then(StemProducer::preparing_wait_ms);
                    set_playback_state(&state, |s| {
                        s.status = PlaybackStatus::Buffering;
                        s.preparing_wait_ms = wait_ms;
                    });
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
    })();
    if running {
        let _ = output.stop();
    }
    drop(producer);
    result
}
