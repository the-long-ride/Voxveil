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
use wasapi::{Device, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

use crate::device::EndpointDescriptor;
use crate::virtual_endpoint::classify_virtual_endpoint;

use super::{
    MAX_QUEUE_FRAMES, MAX_RENDER_FRAMES, OwnedPlaybackSnapshot, PLAYBACK_DECODE_FRAMES,
    PlaybackCommand, PlaybackStatus, WASAPI_BUFFER_HNS, set_playback_state,
};
fn physical_render_device(
    enumerator: &DeviceEnumerator,
    endpoint: &EndpointDescriptor,
) -> Result<Device, String> {
    if classify_virtual_endpoint(endpoint).is_some() {
        return Err("a virtual audio endpoint cannot be used for local file playback".into());
    }
    let collection = enumerator
        .get_device_collection(&Direction::Render)
        .map_err(|error| error.to_string())?;
    for device in &collection {
        let device = device.map_err(|error| error.to_string())?;
        let id = device.get_id().map_err(|error| error.to_string())?;
        if id != endpoint.id {
            continue;
        }
        let actual = EndpointDescriptor {
            id,
            name: device
                .get_friendlyname()
                .map_err(|error| error.to_string())?,
            interface_name: device.get_interface_friendlyname().ok(),
            description: device.get_description().ok(),
            is_default: endpoint.is_default,
        };
        if classify_virtual_endpoint(&actual).is_some() {
            return Err("a virtual audio endpoint cannot be used for local file playback".into());
        }
        return Ok(device);
    }
    Err("the selected physical playback endpoint is no longer available".into())
}

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        wasapi::deinitialize();
    }
}

pub(super) fn run_playback_worker(
    path: PathBuf,
    endpoint: EndpointDescriptor,
    initial_vocal_level: u8,
    initial_profile: ClassicSuppressionProfile,
    commands: Receiver<PlaybackCommand>,
    state: Arc<Mutex<OwnedPlaybackSnapshot>>,
) -> Result<(), String> {
    wasapi::initialize_mta()
        .ok()
        .map_err(|error| error.to_string())?;
    let _com = ComGuard;

    let mut decoder = MediaDecoder::open(&path).map_err(|error| error.to_string())?;
    let info = decoder.info();
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let device = physical_render_device(&enumerator, &endpoint)?;
    let format = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        info.sample_rate as usize,
        2,
        None,
    );
    let mut client = device
        .get_iaudioclient()
        .map_err(|error| error.to_string())?;
    let mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns: WASAPI_BUFFER_HNS,
    };
    client
        .initialize_client(&format, &Direction::Render, &mode)
        .map_err(|error| error.to_string())?;
    let render = client
        .get_audiorenderclient()
        .map_err(|error| error.to_string())?;
    let mut processor = new_processor(info.sample_rate, initial_vocal_level, initial_profile)?;
    let mut current_vocal_level = initial_vocal_level;
    let mut current_profile = initial_profile;
    let mut queue = VecDeque::<u8>::with_capacity(MAX_QUEUE_FRAMES * 8);
    let mut pcm = Vec::<f32>::with_capacity(PLAYBACK_DECODE_FRAMES * 2);
    let mut silence = Vec::<u8>::with_capacity(MAX_RENDER_FRAMES * 8);
    let mut paused = false;
    let mut source_eof = false;
    let mut tail_flushed = false;

    client.start_stream().map_err(|error| error.to_string())?;
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
                match commands.try_recv() {
                    Ok(PlaybackCommand::Stop) => break 'playback Ok(()),
                    Ok(PlaybackCommand::Pause) if !paused => {
                        client.stop_stream().map_err(|error| error.to_string())?;
                        paused = true;
                        set_playback_state(&state, |snapshot| {
                            snapshot.status = PlaybackStatus::Paused
                        });
                    }
                    Ok(PlaybackCommand::Resume) if paused => {
                        client.start_stream().map_err(|error| error.to_string())?;
                        paused = false;
                        set_playback_state(&state, |snapshot| {
                            snapshot.status = PlaybackStatus::Playing
                        });
                    }
                    Ok(PlaybackCommand::Seek(frame)) => {
                        let total = info.total_frames.unwrap_or(frame);
                        let target = frame.min(total);
                        client.stop_stream().map_err(|error| error.to_string())?;
                        client.reset_stream().map_err(|error| error.to_string())?;
                        queue.clear();
                        processor =
                            new_processor(info.sample_rate, current_vocal_level, current_profile)?;
                        let actual = decoder.seek(target).map_err(|error| error.to_string())?;
                        source_eof = false;
                        tail_flushed = false;
                        if !paused {
                            client.start_stream().map_err(|error| error.to_string())?;
                        }
                        set_playback_state(&state, |snapshot| {
                            snapshot.position_frames = actual;
                            snapshot.status = if paused {
                                PlaybackStatus::Paused
                            } else {
                                PlaybackStatus::Playing
                            };
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
                    Ok(_) | Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break 'playback Ok(()),
                }
            }

            if paused {
                thread::sleep(Duration::from_millis(10));
                continue;
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

            let available = client
                .get_available_space_in_frames()
                .map_err(|error| error.to_string())? as usize;
            if available > MAX_RENDER_FRAMES {
                break Err(format!(
                    "WASAPI requested an unbounded render buffer: {available} frames"
                ));
            }
            let frames_to_write = available.min(queue.len() / 8);
            if frames_to_write > 0 {
                render
                    .write_to_device_from_deque(frames_to_write, &mut queue, None)
                    .map_err(|error| error.to_string())?;
            } else if available > 0 && !source_eof {
                silence.clear();
                silence.resize(available * 8, 0);
                render
                    .write_to_device(available, &silence, None)
                    .map_err(|error| error.to_string())?;
            }

            let device_pending = client
                .get_current_padding()
                .map_err(|error| error.to_string())? as u64;
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
                set_playback_state(&state, |snapshot| {
                    snapshot.status = PlaybackStatus::Ended;
                    snapshot.position_frames =
                        info.total_frames.unwrap_or(decoder.position_frames());
                });
                break 'playback Ok(());
            }
            thread::sleep(Duration::from_millis(2));
        }
    })();

    let stop_result = client.stop_stream().map_err(|error| error.to_string());
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
