use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

use crate::device::{
    BackendProbe, EndpointDescriptor, RelayReadiness, choose_endpoints, find_virtual_render,
    is_virtual_render_name,
};
use crate::process_f32le_stereo;

const SAMPLE_RATE: usize = 48_000;
const CHANNELS: usize = 2;
const BYTES_PER_FRAME: usize = 8;
const BUFFER_DURATION_HNS: i64 = 200_000;
const MAX_QUEUE_BYTES: usize = SAMPLE_RATE * BYTES_PER_FRAME;

#[derive(Clone)]
struct RelayControl {
    stop: Arc<AtomicBool>,
    processing_enabled: Arc<AtomicBool>,
    vocal_percent: Arc<AtomicU8>,
}

impl RelayControl {
    fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            processing_enabled: Arc::new(AtomicBool::new(false)),
            vocal_percent: Arc::new(AtomicU8::new(100)),
        }
    }
}

#[derive(Clone, Debug)]
struct RelayRoute {
    virtual_render_id: String,
    physical_id: String,
}

pub struct WindowsAudioBackend {
    control: RelayControl,
    worker: Option<JoinHandle<()>>,
    last_error: Arc<Mutex<Option<String>>>,
    preferred_physical_id: Option<String>,
}

impl WindowsAudioBackend {
    pub fn new() -> Self {
        Self {
            control: RelayControl::new(),
            worker: None,
            last_error: Arc::new(Mutex::new(None)),
            preferred_physical_id: None,
        }
    }

    pub fn probe(&mut self) -> BackendProbe {
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
            if let Ok(error) = self.last_error.lock() {
                if let Some(detail) = error.clone() {
                    return BackendProbe {
                        readiness: RelayReadiness::Faulted,
                        physical_output: None,
                        detail: Some(detail),
                    };
                }
            }
        }
        match probe_blocking(self.preferred_physical_id.as_deref()) {
            Ok((route, probe)) if probe.readiness == RelayReadiness::Ready => {
                if self.worker.is_none() {
                    self.control
                        .processing_enabled
                        .store(false, Ordering::Release);
                    if let Err(error) = self.start_worker(route) {
                        return fault_probe(error);
                    }
                }
                probe
            }
            Ok((route, probe)) => {
                if probe.readiness == RelayReadiness::RoutingRequired
                    && !route.physical_id.is_empty()
                {
                    self.preferred_physical_id = Some(route.physical_id);
                }
                probe
            }
            Err(error) => fault_probe(error),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool, vocal_level: u8) -> Result<BackendProbe, String> {
        self.set_vocal_level(vocal_level);
        let (route, probe) = probe_blocking(self.preferred_physical_id.as_deref())?;
        if probe.readiness != RelayReadiness::Ready {
            self.control
                .processing_enabled
                .store(false, Ordering::Release);
            self.stop();
            return if enabled {
                Err(probe
                    .detail
                    .clone()
                    .unwrap_or_else(|| "Windows audio relay is not ready".into()))
            } else {
                Ok(probe)
            };
        }
        if self.worker.as_ref().is_none_or(JoinHandle::is_finished) {
            self.stop();
            self.start_worker(route)?;
        }
        self.control
            .processing_enabled
            .store(enabled, Ordering::Release);
        Ok(probe)
    }

    fn start_worker(&mut self, route: RelayRoute) -> Result<(), String> {
        self.control.stop.store(false, Ordering::Release);
        if let Ok(mut error) = self.last_error.lock() {
            *error = None;
        }
        let control = self.control.clone();
        let last_error = self.last_error.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let handle = thread::Builder::new()
            .name("voxveil-wasapi-relay".into())
            .spawn(move || {
                let result = run_relay(route, control, ready_tx);
                if let Err(message) = result {
                    if let Ok(mut error) = last_error.lock() {
                        *error = Some(message);
                    }
                }
            })
            .map_err(|error| format!("failed to start Windows audio relay: {error}"))?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                self.worker = Some(handle);
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = handle.join();
                Err(error)
            }
            Err(_) => {
                self.control.stop.store(true, Ordering::Release);
                let _ = handle.join();
                Err("Windows audio relay timed out while starting".into())
            }
        }
    }

    pub fn set_vocal_level(&self, value: u8) {
        self.control
            .vocal_percent
            .store(value.min(100), Ordering::Release);
    }

    pub fn physical_outputs(&self) -> Vec<String> {
        enumerate_direction_blocking(Direction::Render)
            .map(|items| {
                items
                    .into_iter()
                    .filter(|item| !is_virtual_render_name(&item.name))
                    .map(|item| item.name)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn stop(&mut self) {
        self.control.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for WindowsAudioBackend {
    fn drop(&mut self) {
        self.stop();
    }
}

fn fault_probe(error: String) -> BackendProbe {
    BackendProbe {
        readiness: RelayReadiness::Faulted,
        physical_output: None,
        detail: Some(error),
    }
}

fn probe_blocking(
    preferred_physical_id: Option<&str>,
) -> Result<(RelayRoute, BackendProbe), String> {
    let preferred = preferred_physical_id.map(str::to_owned);
    thread::spawn(move || {
        wasapi::initialize_mta()
            .ok()
            .map_err(|error| error.to_string())?;
        let result = (|| {
            let render = enumerate_direction_inner(Direction::Render)?;
            let mut probe = choose_endpoints(&render);
            let virtual_render_id = find_virtual_render(&render)
                .map(|item| item.id.clone())
                .unwrap_or_default();
            let physical = preferred
                .as_deref()
                .and_then(|id| {
                    render
                        .iter()
                        .find(|item| item.id == id && !is_virtual_render_name(&item.name))
                })
                .or_else(|| {
                    render
                        .iter()
                        .find(|item| item.is_default && !is_virtual_render_name(&item.name))
                })
                .or_else(|| {
                    render
                        .iter()
                        .find(|item| !is_virtual_render_name(&item.name))
                });
            let physical_id = physical.map(|item| item.id.clone()).unwrap_or_default();
            if let Some(endpoint) = physical {
                probe.physical_output = Some(endpoint.name.clone());
            }
            Ok((
                RelayRoute {
                    virtual_render_id,
                    physical_id,
                },
                probe,
            ))
        })();
        wasapi::deinitialize();
        result
    })
    .join()
    .map_err(|_| "Windows endpoint probe panicked".to_string())?
}

fn enumerate_direction_blocking(direction: Direction) -> Result<Vec<EndpointDescriptor>, String> {
    thread::spawn(move || {
        wasapi::initialize_mta()
            .ok()
            .map_err(|error| error.to_string())?;
        let result = enumerate_direction_inner(direction);
        wasapi::deinitialize();
        result
    })
    .join()
    .map_err(|_| "Windows endpoint enumeration panicked".to_string())?
}

fn enumerate_direction_inner(direction: Direction) -> Result<Vec<EndpointDescriptor>, String> {
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let default_id = enumerator
        .get_default_device(&direction)
        .and_then(|device| device.get_id())
        .unwrap_or_default();
    let collection = enumerator
        .get_device_collection(&direction)
        .map_err(|error| error.to_string())?;
    let mut endpoints = Vec::new();
    for device in &collection {
        let device = device.map_err(|error| error.to_string())?;
        let id = device.get_id().map_err(|error| error.to_string())?;
        let name = device
            .get_friendlyname()
            .map_err(|error| error.to_string())?;
        endpoints.push(EndpointDescriptor {
            is_default: id == default_id,
            id,
            name,
        });
    }
    Ok(endpoints)
}

fn run_relay(
    route: RelayRoute,
    control: RelayControl,
    ready: mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    wasapi::initialize_mta()
        .ok()
        .map_err(|error| error.to_string())?;
    let result = run_relay_inner(route, control, &ready);
    wasapi::deinitialize();
    result
}

fn run_relay_inner(
    route: RelayRoute,
    control: RelayControl,
    ready: &mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let setup = setup_clients(&route);
    let (capture_client, capture, render_client, render) = match setup {
        Ok(value) => value,
        Err(error) => {
            let _ = ready.send(Err(error.clone()));
            return Err(error);
        }
    };
    capture_client
        .start_stream()
        .map_err(|error| error.to_string())?;
    render_client
        .start_stream()
        .map_err(|error| error.to_string())?;
    let _ = ready.send(Ok(()));

    let mut queue = VecDeque::with_capacity(SAMPLE_RATE * BYTES_PER_FRAME / 4);
    let mut scratch = vec![0_u8; 4096 * BYTES_PER_FRAME];
    while !control.stop.load(Ordering::Acquire) {
        while let Some(frames) = capture
            .get_next_packet_size()
            .map_err(|error| error.to_string())?
        {
            if frames == 0 {
                break;
            }
            let needed = frames as usize * BYTES_PER_FRAME;
            if scratch.len() < needed {
                scratch.resize(needed, 0);
            }
            let (read_frames, _) = capture
                .read_from_device(&mut scratch[..needed])
                .map_err(|error| error.to_string())?;
            let valid = read_frames as usize * BYTES_PER_FRAME;
            if control.processing_enabled.load(Ordering::Acquire) {
                process_f32le_stereo(
                    &mut scratch[..valid],
                    control.vocal_percent.load(Ordering::Acquire),
                )?;
            }
            queue.extend(&scratch[..valid]);
            while queue.len() > MAX_QUEUE_BYTES {
                queue.pop_front();
            }
        }
        let available = render_client
            .get_available_space_in_frames()
            .map_err(|error| error.to_string())? as usize;
        let frames = available.min(queue.len() / BYTES_PER_FRAME);
        if frames > 0 {
            render
                .write_to_device_from_deque(frames, &mut queue, None)
                .map_err(|error| error.to_string())?;
        }
        thread::sleep(Duration::from_millis(2));
    }
    let _ = capture_client.stop_stream();
    let _ = render_client.stop_stream();
    Ok(())
}

fn setup_clients(
    route: &RelayRoute,
) -> Result<
    (
        wasapi::AudioClient,
        wasapi::AudioCaptureClient,
        wasapi::AudioClient,
        wasapi::AudioRenderClient,
    ),
    String,
> {
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let virtual_render_device = enumerator
        .get_device(&route.virtual_render_id)
        .map_err(|error| error.to_string())?;
    let physical_device = enumerator
        .get_device(&route.physical_id)
        .map_err(|error| error.to_string())?;
    let format = WaveFormat::new(32, 32, &SampleType::Float, SAMPLE_RATE, CHANNELS, None);
    let mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns: BUFFER_DURATION_HNS,
    };

    // WASAPI loopback capture runs against a render endpoint. This lets the
    // virtual driver expose only Voxveil Output; no companion capture endpoint
    // is required.
    let mut capture_client = virtual_render_device
        .get_iaudioclient()
        .map_err(|error| error.to_string())?;
    capture_client
        .initialize_client(&format, &Direction::Capture, &mode)
        .map_err(|error| error.to_string())?;
    let capture = capture_client
        .get_audiocaptureclient()
        .map_err(|error| error.to_string())?;

    let mut render_client = physical_device
        .get_iaudioclient()
        .map_err(|error| error.to_string())?;
    render_client
        .initialize_client(&format, &Direction::Render, &mode)
        .map_err(|error| error.to_string())?;
    let render = render_client
        .get_audiorenderclient()
        .map_err(|error| error.to_string())?;
    Ok((capture_client, capture, render_client, render))
}
