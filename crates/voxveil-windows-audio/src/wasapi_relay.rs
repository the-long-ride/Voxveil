use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, TryRecvError},
};
use std::time::Duration;

use voxveil_dsp::SpectralCenterSuppressor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};
use wasapi::{Device, DeviceEnumerator, Direction, SampleType, StreamMode};

use crate::relay_buffer::{
    CaptureTimeline, DEVICE_BUFFER_MS, FRAME_BYTES, MAX_CAPTURE_FRAMES, RelayBuffer, RenderClock,
};
use crate::relay_engine::{RelayCommand, RelayRuntimeState, RelaySpec};
use crate::sample::process_capture_f32le_stereo;

const MAX_PACKETS_PER_TICK: usize = 16;

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        wasapi::deinitialize();
    }
}

fn relay_format_supported(channels: u16, bits: u16, is_float: bool) -> bool {
    channels == 2 && bits == 32 && is_float
}

fn capture_packet_within_bound(frames: usize) -> bool {
    frames <= MAX_CAPTURE_FRAMES
}

fn set_state(state: &Arc<Mutex<RelayRuntimeState>>, next: RelayRuntimeState) {
    if let Ok(mut state) = state.lock() {
        *state = next;
    }
}

fn active_render_device_by_id(
    enumerator: &DeviceEnumerator,
    wanted_id: &str,
) -> Result<Device, String> {
    let collection = enumerator
        .get_device_collection(&Direction::Render)
        .map_err(|error| error.to_string())?;

    for device in &collection {
        let device = device.map_err(|error| error.to_string())?;
        let id = device.get_id().map_err(|error| error.to_string())?;
        if id == wanted_id {
            return Ok(device);
        }
    }

    Err(format!("active render endpoint not found: {wanted_id}"))
}

pub(crate) fn run_relay_worker_with_profile(
    spec: RelaySpec,
    initial_vocal_level: u8,
    initial_profile: ClassicSuppressionProfile,
    control_rx: Receiver<RelayCommand>,
    state: Arc<Mutex<RelayRuntimeState>>,
) -> Result<(), String> {
    let result = run_relay_worker_inner(
        spec,
        initial_vocal_level,
        initial_profile,
        control_rx,
        &state,
    );
    match &result {
        Ok(()) => set_state(&state, RelayRuntimeState::Stopped),
        Err(error) => set_state(&state, RelayRuntimeState::Faulted(error.clone())),
    }
    result
}

fn run_relay_worker_inner(
    spec: RelaySpec,
    initial_vocal_level: u8,
    initial_profile: ClassicSuppressionProfile,
    control_rx: Receiver<RelayCommand>,
    state: &Arc<Mutex<RelayRuntimeState>>,
) -> Result<(), String> {
    wasapi::initialize_mta()
        .ok()
        .map_err(|error| error.to_string())?;
    let _com = ComGuard;

    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let source_device = active_render_device_by_id(&enumerator, &spec.source_endpoint_id)?;
    let physical_device =
        active_render_device_by_id(&enumerator, &spec.physical_output_endpoint_id)?;

    let mut capture_client = source_device
        .get_iaudioclient()
        .map_err(|error| error.to_string())?;
    let source_format = capture_client
        .get_mixformat()
        .map_err(|error| error.to_string())?;
    let sample_type = source_format
        .get_subformat()
        .map_err(|error| error.to_string())?;
    if !relay_format_supported(
        source_format.get_nchannels(),
        source_format.get_bitspersample(),
        sample_type == SampleType::Float,
    ) || source_format.get_blockalign() as usize != FRAME_BYTES
    {
        return Err(
            "The Windows source output shared format must be stereo 32-bit float. Select that format in Windows Sound device properties or choose another spare output".into(),
        );
    }

    let capture_mode = StreamMode::PollingShared {
        autoconvert: false,
        buffer_duration_hns: DEVICE_BUFFER_MS as i64 * 10_000,
    };
    capture_client
        .initialize_client(&source_format, &Direction::Capture, &capture_mode)
        .map_err(|error| error.to_string())?;
    let capture = capture_client
        .get_audiocaptureclient()
        .map_err(|error| error.to_string())?;

    let mut render_client = physical_device
        .get_iaudioclient()
        .map_err(|error| error.to_string())?;
    let render_mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns: DEVICE_BUFFER_MS as i64 * 10_000,
    };
    render_client
        .initialize_client(&source_format, &Direction::Render, &render_mode)
        .map_err(|error| error.to_string())?;
    let render = render_client
        .get_audiorenderclient()
        .map_err(|error| error.to_string())?;
    let render_clock = RenderClock::new(&render_client);
    let render_capacity = render_client
        .get_buffer_size()
        .map_err(|error| error.to_string())? as usize;
    let sample_rate = source_format.get_samplespersec();
    let mut queue = RelayBuffer::new(render_capacity, sample_rate);
    let mut timeline = CaptureTimeline::default();
    let mut capture_buffer = vec![0_u8; MAX_CAPTURE_FRAMES * FRAME_BYTES];
    let mut gap_buffer = vec![0_u8; MAX_CAPTURE_FRAMES * FRAME_BYTES];
    let initial_level =
        VocalLevel::new(initial_vocal_level.min(100) as f32 / 100.0).map_err(str::to_string)?;
    let mut processor = SpectralCenterSuppressor::new(sample_rate, initial_level, initial_profile);

    render
        .write_to_device(
            render_capacity,
            &vec![0; render_capacity * FRAME_BYTES],
            None,
        )
        .map_err(|error| error.to_string())?;
    render_client
        .start_stream()
        .map_err(|error| error.to_string())?;
    if let Err(error) = capture_client.start_stream() {
        let _ = render_client.stop_stream();
        return Err(error.to_string());
    }

    set_state(state, RelayRuntimeState::Running);

    let loop_result: Result<(), String> = 'relay: loop {
        loop {
            match control_rx.try_recv() {
                Ok(RelayCommand::SetVocalLevel(value)) => {
                    let level =
                        VocalLevel::new(value.min(100) as f32 / 100.0).map_err(str::to_string)?;
                    processor.set_vocal_level(level);
                }
                Ok(RelayCommand::SetSuppressionProfile(profile)) => {
                    processor.set_profile(profile);
                }
                Ok(RelayCommand::Stop) => break 'relay Ok(()),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break 'relay Ok(()),
            }
        }

        let mut capture_drained = false;
        let mut captured_frames = 0;
        for _ in 0..MAX_PACKETS_PER_TICK {
            let frame_count = match capture.get_next_packet_size() {
                Ok(Some(frames)) if frames > 0 => frames as usize,
                Ok(_) => {
                    capture_drained = true;
                    break;
                }
                Err(error) => break 'relay Err(error.to_string()),
            };
            if !capture_packet_within_bound(frame_count) {
                break 'relay Err(format!(
                    "relay capture packet exceeded bounded buffer: {frame_count} frames"
                ));
            }
            let bytes = frame_count * FRAME_BYTES;
            let (read_frames, info) = match capture.read_from_device(&mut capture_buffer[..bytes]) {
                Ok(value) => value,
                Err(error) => break 'relay Err(error.to_string()),
            };
            let read_frames = read_frames as usize;
            if read_frames > frame_count {
                break 'relay Err(
                    "WASAPI capture returned more frames than the announced packet".into(),
                );
            }
            if read_frames == 0 {
                capture_drained = true;
                break;
            }
            let gap = timeline.missing_frames(
                info.index,
                read_frames,
                info.flags.data_discontinuity,
                info.flags.timestamp_error,
            );
            if gap != 0 || info.flags.data_discontinuity || info.flags.timestamp_error {
                queue.reset_drift();
            } else {
                queue.observe_clocks(info.index, info.timestamp, || render_clock.position());
            }
            for (buffer, silent) in [
                (&mut gap_buffer[..gap * FRAME_BYTES], true),
                (
                    &mut capture_buffer[..read_frames * FRAME_BYTES],
                    info.flags.silent,
                ),
            ] {
                if let Err(error) = process_capture_f32le_stereo(buffer, silent, &mut processor) {
                    break 'relay Err(error.to_string());
                }
                queue.push_capture(buffer);
            }
            captured_frames += read_frames;
        }

        let writable = match render_client.get_available_space_in_frames() {
            Ok(frames) => frames as usize,
            Err(error) => break Err(error.to_string()),
        };
        queue.balance(render_capacity - writable, captured_frames);
        let frames_to_write = writable.min(queue.queued_frames());
        if frames_to_write > 0 {
            if let Err(error) =
                render.write_to_device_from_deque(frames_to_write, queue.render_queue(), None)
            {
                break Err(error.to_string());
            }
        }

        if capture_drained {
            std::thread::sleep(Duration::from_millis(1));
        }
    };

    let capture_stop = capture_client
        .stop_stream()
        .map_err(|error| error.to_string());
    let render_stop = render_client
        .stop_stream()
        .map_err(|error| error.to_string());

    loop_result?;
    capture_stop?;
    render_stop?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_dsp_requires_stereo_f32() {
        assert!(relay_format_supported(2, 32, true));
        assert!(!relay_format_supported(1, 32, true));
        assert!(!relay_format_supported(2, 16, false));
        assert!(!relay_format_supported(2, 64, true));
    }

    #[test]
    fn relay_capture_packet_has_a_fixed_memory_bound() {
        assert!(capture_packet_within_bound(MAX_CAPTURE_FRAMES));
        assert!(!capture_packet_within_bound(MAX_CAPTURE_FRAMES + 1));
    }
}
