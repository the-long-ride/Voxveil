use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, TryRecvError},
};
use std::time::Duration;

use voxveil_dsp::SpectralCenterSuppressor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};
use wasapi::{Device, DeviceEnumerator, Direction, SampleType, StreamMode};

use crate::relay_engine::{RelayCommand, RelayRuntimeState, RelaySpec};
use crate::sample::process_capture_f32le_stereo;

const MAX_CAPTURE_FRAMES: usize = 4096;
const MAX_QUEUE_FRAMES: usize = 8192;

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

fn queue_append_within_bound(current_bytes: usize, incoming_bytes: usize, max_bytes: usize) -> bool {
    current_bytes
        .checked_add(incoming_bytes)
        .is_some_and(|total| total <= max_bytes)
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

pub(crate) fn run_relay_worker(
    spec: RelaySpec,
    initial_vocal_level: u8,
    control_rx: Receiver<RelayCommand>,
    state: Arc<Mutex<RelayRuntimeState>>,
) -> Result<(), String> {
    run_relay_worker_with_profile(
        spec,
        initial_vocal_level,
        ClassicSuppressionProfile::default(),
        control_rx,
        state,
    )
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
    ) {
        return Err(
            "virtual interception endpoint shared format must be stereo 32-bit float for this relay version".into(),
        );
    }

    let capture_mode = StreamMode::PollingShared {
        autoconvert: false,
        buffer_duration_hns: 200_000,
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
        buffer_duration_hns: 200_000,
    };
    render_client
        .initialize_client(&source_format, &Direction::Render, &render_mode)
        .map_err(|error| error.to_string())?;
    let render = render_client
        .get_audiorenderclient()
        .map_err(|error| error.to_string())?;

    render_client
        .start_stream()
        .map_err(|error| error.to_string())?;
    if let Err(error) = capture_client.start_stream() {
        let _ = render_client.stop_stream();
        return Err(error.to_string());
    }

    set_state(state, RelayRuntimeState::Running);

    let bytes_per_frame = source_format.get_blockalign() as usize;
    let max_queue_bytes = bytes_per_frame
        .checked_mul(MAX_QUEUE_FRAMES)
        .ok_or_else(|| "relay queue size overflow".to_string())?;
    let capture_buffer_bytes = bytes_per_frame
        .checked_mul(MAX_CAPTURE_FRAMES)
        .ok_or_else(|| "relay capture buffer size overflow".to_string())?;
    let mut queue = VecDeque::<u8>::with_capacity(max_queue_bytes);
    let mut capture_buffer = vec![0_u8; capture_buffer_bytes];
    let initial_level = VocalLevel::new(initial_vocal_level.min(100) as f32 / 100.0)
        .map_err(str::to_string)?;
    let mut processor = SpectralCenterSuppressor::new(
        source_format.get_samplespersec(),
        initial_level,
        initial_profile,
    );

    let loop_result: Result<(), String> = 'relay: loop {
        loop {
            match control_rx.try_recv() {
                Ok(RelayCommand::SetVocalLevel(value)) => {
                    let level = VocalLevel::new(value.min(100) as f32 / 100.0)
                        .map_err(str::to_string)?;
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

        match capture
            .get_next_packet_size()
            .map_err(|error| error.to_string())
        {
            Ok(Some(frames)) if frames > 0 => {
                let frame_count = frames as usize;
                if !capture_packet_within_bound(frame_count) {
                    break Err(format!(
                        "relay capture packet exceeded bounded buffer: {frame_count} frames"
                    ));
                }
                let bytes = match frame_count.checked_mul(bytes_per_frame) {
                    Some(bytes) => bytes,
                    None => break Err("relay capture packet size overflow".into()),
                };
                let (read_frames, info) = match capture.read_from_device(&mut capture_buffer[..bytes]) {
                    Ok(value) => value,
                    Err(error) => break Err(error.to_string()),
                };
                let read_bytes = match (read_frames as usize).checked_mul(bytes_per_frame) {
                    Some(bytes) => bytes,
                    None => break Err("relay captured byte count overflow".into()),
                };
                if read_bytes > bytes {
                    break Err("WASAPI capture returned more frames than the announced packet".into());
                }
                if let Err(error) = process_capture_f32le_stereo(
                    &mut capture_buffer[..read_bytes],
                    info.flags.silent,
                    &mut processor,
                ) {
                    break Err(error.to_string());
                }
                if !queue_append_within_bound(queue.len(), read_bytes, max_queue_bytes) {
                    break Err("relay buffer exceeded the bounded latency limit".into());
                }
                queue.extend(&capture_buffer[..read_bytes]);
            }
            Ok(_) => {}
            Err(error) => break Err(error),
        }

        let writable = match render_client.get_available_space_in_frames() {
            Ok(frames) => frames as usize,
            Err(error) => break Err(error.to_string()),
        };
        let queued_frames = queue.len() / bytes_per_frame;
        let frames_to_write = writable.min(queued_frames);
        if frames_to_write > 0 {
            if let Err(error) = render.write_to_device_from_deque(frames_to_write, &mut queue, None)
            {
                break Err(error.to_string());
            }
        }

        std::thread::sleep(Duration::from_millis(2));
    };

    let capture_stop = capture_client.stop_stream().map_err(|error| error.to_string());
    let render_stop = render_client.stop_stream().map_err(|error| error.to_string());

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

    #[test]
    fn relay_queue_rejects_append_before_capacity_growth() {
        assert!(queue_append_within_bound(64, 32, 96));
        assert!(!queue_append_within_bound(65, 32, 96));
        assert!(!queue_append_within_bound(usize::MAX, 1, usize::MAX));
    }
}
