use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Sender},
};
use std::thread::{self, JoinHandle};

use voxveil_types::{AudioPlaybackSnapshot, AudioPlaybackStatus, ClassicSuppressionProfile};

use crate::device::EndpointDescriptor;
use crate::virtual_endpoint::classify_virtual_endpoint;

const PLAYBACK_DECODE_FRAMES: usize = 2_048;
const MAX_QUEUE_FRAMES: usize = 8_192;
const MAX_RENDER_FRAMES: usize = 8_192;
const WASAPI_BUFFER_HNS: i64 = 200_000;

#[path = "playback_worker.rs"]
mod playback_worker;
use playback_worker::run_playback_worker;

pub type OwnedPlaybackSnapshot = AudioPlaybackSnapshot;
pub type PlaybackStatus = AudioPlaybackStatus;

#[derive(Clone, Debug, PartialEq, Eq)]
enum PlaybackCommand {
    Pause,
    Resume,
    Seek(u64),
    Stop,
    SetVocalLevel(u8),
    SetProfile(ClassicSuppressionProfile),
}

struct PlaybackHandle {
    command_tx: Sender<PlaybackCommand>,
    state: Arc<Mutex<OwnedPlaybackSnapshot>>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Default)]
pub struct OwnedPlayback {
    handle: Option<PlaybackHandle>,
}

impl OwnedPlayback {
    pub fn open(
        &mut self,
        path: PathBuf,
        endpoint: EndpointDescriptor,
        vocal_level: u8,
        profile: ClassicSuppressionProfile,
    ) -> Result<OwnedPlaybackSnapshot, String> {
        self.stop()?;
        if classify_virtual_endpoint(&endpoint).is_some() {
            return Err("a virtual audio endpoint cannot be used for local file playback".into());
        }
        if path.file_name().is_none() {
            return Err("select a local audio file before starting playback".into());
        }
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        let state = Arc::new(Mutex::new(OwnedPlaybackSnapshot {
            status: PlaybackStatus::Loading,
            file_name,
            output_endpoint_id: Some(endpoint.id.clone()),
            output_name: Some(endpoint.name.clone()),
            ..OwnedPlaybackSnapshot::default()
        }));
        let worker_state = Arc::clone(&state);
        let exit_state = Arc::clone(&state);
        let (command_tx, command_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_playback_worker(
                    path,
                    endpoint,
                    vocal_level,
                    profile,
                    command_rx,
                    worker_state,
                )
            }));
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => set_playback_state(&exit_state, |snapshot| {
                    snapshot.status = PlaybackStatus::Faulted;
                    snapshot.error = Some(error);
                }),
                Err(_) => set_playback_state(&exit_state, |snapshot| {
                    snapshot.status = PlaybackStatus::Faulted;
                    snapshot.error =
                        Some("local audio playback worker stopped unexpectedly".into());
                }),
            }
        });
        self.handle = Some(PlaybackHandle {
            command_tx,
            state: Arc::clone(&state),
            worker: Some(worker),
        });
        Ok(snapshot_from(&state))
    }

    pub fn snapshot(&self) -> OwnedPlaybackSnapshot {
        self.handle
            .as_ref()
            .map(|handle| snapshot_from(&handle.state))
            .unwrap_or_default()
    }

    pub fn pause(&self) -> Result<(), String> {
        self.send(PlaybackCommand::Pause)
    }

    pub fn resume(&self) -> Result<(), String> {
        self.send(PlaybackCommand::Resume)
    }

    pub fn seek(&self, frame: u64) -> Result<(), String> {
        self.send(PlaybackCommand::Seek(frame))
    }

    pub fn set_vocal_level(&self, value: u8) -> Result<(), String> {
        self.send(PlaybackCommand::SetVocalLevel(value.min(100)))
    }

    pub fn set_profile(&self, profile: ClassicSuppressionProfile) -> Result<(), String> {
        self.send(PlaybackCommand::SetProfile(profile))
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let Some(handle) = self.handle.as_mut() else {
            return Ok(());
        };
        let _ = handle.command_tx.send(PlaybackCommand::Stop);
        if let Some(worker) = handle.worker.take() {
            worker
                .join()
                .map_err(|_| "local audio playback worker wrapper panicked".to_string())?;
        }
        set_playback_state(&handle.state, |snapshot| {
            if matches!(
                snapshot.status,
                PlaybackStatus::Loading | PlaybackStatus::Playing | PlaybackStatus::Paused
            ) {
                snapshot.status = PlaybackStatus::Stopped;
            }
        });
        Ok(())
    }

    fn send(&self, command: PlaybackCommand) -> Result<(), String> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| "no local file is open for playback".to_string())?;
        handle
            .command_tx
            .send(command)
            .map_err(|_| "local audio playback worker is not running".to_string())
    }
}

impl Drop for OwnedPlayback {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn snapshot_from(state: &Arc<Mutex<OwnedPlaybackSnapshot>>) -> OwnedPlaybackSnapshot {
    match state.lock() {
        Ok(snapshot) => snapshot.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

fn set_playback_state(
    state: &Arc<Mutex<OwnedPlaybackSnapshot>>,
    update: impl FnOnce(&mut OwnedPlaybackSnapshot),
) {
    let mut snapshot = match state.lock() {
        Ok(snapshot) => snapshot,
        Err(poisoned) => poisoned.into_inner(),
    };
    update(&mut snapshot);
}

#[cfg(test)]
mod tests {
    use super::playback_worker::enqueue_processed;
    use super::*;
    use std::collections::VecDeque;

    fn endpoint(id: &str, name: &str, interface: &str, description: &str) -> EndpointDescriptor {
        EndpointDescriptor {
            id: id.into(),
            name: name.into(),
            interface_name: Some(interface.into()),
            description: Some(description.into()),
            is_default: false,
        }
    }

    #[test]
    fn queue_rejects_growth_past_its_fixed_capacity() {
        let mut queue = VecDeque::with_capacity(MAX_QUEUE_FRAMES * 8);
        let input = vec![0.25; MAX_QUEUE_FRAMES * 2];
        enqueue_processed(&mut queue, &input).unwrap();
        assert_eq!(queue.len(), MAX_QUEUE_FRAMES * 8);
        assert!(enqueue_processed(&mut queue, &[0.0, 0.0]).is_err());
    }

    #[test]
    fn only_a_physical_endpoint_can_own_file_playback() {
        assert!(
            classify_virtual_endpoint(&endpoint("speakers", "Speakers", "USB Audio", "Speakers"))
                .is_none()
        );
        assert!(
            classify_virtual_endpoint(&endpoint(
                "cable",
                "CABLE Input (VB-Audio Virtual Cable)",
                "VB-Audio Virtual Cable",
                "CABLE Input",
            ))
            .is_some()
        );
        assert!(
            classify_virtual_endpoint(&endpoint(
                "voxveil",
                "Voxveil Input",
                "Voxveil Virtual Audio",
                "Voxveil Input",
            ))
            .is_some()
        );
    }
}
