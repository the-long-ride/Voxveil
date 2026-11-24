use std::collections::VecDeque;

use wasapi::{
    AudioClient, AudioRenderClient, Device, DeviceEnumerator, Direction, SampleType, StreamMode,
    WaveFormat,
};

use crate::device::EndpointDescriptor;
use crate::virtual_endpoint::is_virtual_endpoint;

const MAX_RENDER_FRAMES: usize = 8_192;
const WASAPI_BUFFER_HNS: i64 = 200_000;

pub(super) trait PlaybackOutput {
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn reset(&mut self) -> Result<(), String>;
    fn render(&mut self, queue: &mut VecDeque<u8>, source_eof: bool) -> Result<u64, String>;
}

pub(super) struct WasapiOutput {
    render: AudioRenderClient,
    client: AudioClient,
    silence: Vec<u8>,
    _com: ComGuard,
}

impl WasapiOutput {
    pub(super) fn open(endpoint: &EndpointDescriptor, sample_rate: u32) -> Result<Self, String> {
        wasapi::initialize_mta()
            .ok()
            .map_err(|error| error.to_string())?;
        let com = ComGuard;
        let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
        let device = physical_render_device(&enumerator, endpoint)?;
        let format = WaveFormat::new(32, 32, &SampleType::Float, sample_rate as usize, 2, None);
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
        Ok(Self {
            render,
            client,
            silence: Vec::with_capacity(MAX_RENDER_FRAMES * 8),
            _com: com,
        })
    }
}

impl PlaybackOutput for WasapiOutput {
    fn start(&mut self) -> Result<(), String> {
        self.client
            .start_stream()
            .map_err(|error| error.to_string())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.client.stop_stream().map_err(|error| error.to_string())
    }

    fn reset(&mut self) -> Result<(), String> {
        self.client
            .reset_stream()
            .map_err(|error| error.to_string())
    }

    fn render(&mut self, queue: &mut VecDeque<u8>, source_eof: bool) -> Result<u64, String> {
        let available = self
            .client
            .get_available_space_in_frames()
            .map_err(|error| error.to_string())? as usize;
        if available > MAX_RENDER_FRAMES {
            return Err(format!(
                "WASAPI requested an unbounded render buffer: {available} frames"
            ));
        }
        let frames_to_write = available.min(queue.len() / 8);
        if frames_to_write > 0 {
            self.render
                .write_to_device_from_deque(frames_to_write, queue, None)
                .map_err(|error| error.to_string())?;
        } else if available > 0 && !source_eof {
            self.silence.clear();
            self.silence.resize(available * 8, 0);
            self.render
                .write_to_device(available, &self.silence, None)
                .map_err(|error| error.to_string())?;
        }
        self.client
            .get_current_padding()
            .map(u64::from)
            .map_err(|error| error.to_string())
    }
}

fn physical_render_device(
    enumerator: &DeviceEnumerator,
    endpoint: &EndpointDescriptor,
) -> Result<Device, String> {
    if is_virtual_endpoint(endpoint) {
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
        if is_virtual_endpoint(&actual) {
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
