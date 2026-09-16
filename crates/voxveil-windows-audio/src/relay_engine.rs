use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};
use std::thread::{self, JoinHandle};

use voxveil_types::ClassicSuppressionProfile;

use crate::route::validate_distinct_route;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RelaySpec {
    pub source_endpoint_id: String,
    pub physical_output_endpoint_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RelayRuntimeState {
    Stopped,
    Starting,
    Running,
    Faulted(String),
}

impl RelayRuntimeState {
    pub(crate) fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RelayCommand {
    SetVocalLevel(u8),
    SetSuppressionProfile(ClassicSuppressionProfile),
    Stop,
}

pub(crate) struct RelayHandle {
    command_tx: Sender<RelayCommand>,
    state: Arc<Mutex<RelayRuntimeState>>,
    worker: Option<JoinHandle<()>>,
}

impl RelayHandle {
    pub(crate) fn validate_spec(spec: &RelaySpec) -> Result<(), String> {
        validate_distinct_route(
            &spec.source_endpoint_id,
            &spec.physical_output_endpoint_id,
        )
        .map_err(str::to_string)
    }

    pub(crate) fn spawn_with_worker<F>(
        spec: RelaySpec,
        vocal_level: u8,
        worker: F,
    ) -> Result<Self, String>
    where
        F: FnOnce(
                RelaySpec,
                u8,
                Receiver<RelayCommand>,
                Arc<Mutex<RelayRuntimeState>>,
            ) + Send
            + 'static,
    {
        Self::spawn_with_worker_profile(
            spec,
            vocal_level,
            ClassicSuppressionProfile::default(),
            move |spec, level, _profile, commands, state| {
                worker(spec, level, commands, state);
            },
        )
    }

    pub(crate) fn spawn_with_worker_profile<F>(
        spec: RelaySpec,
        vocal_level: u8,
        profile: ClassicSuppressionProfile,
        worker: F,
    ) -> Result<Self, String>
    where
        F: FnOnce(
                RelaySpec,
                u8,
                ClassicSuppressionProfile,
                Receiver<RelayCommand>,
                Arc<Mutex<RelayRuntimeState>>,
            ) + Send
            + 'static,
    {
        Self::validate_spec(&spec)?;
        let (command_tx, command_rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(RelayRuntimeState::Starting));
        let worker_state = Arc::clone(&state);
        let exit_state = Arc::clone(&state);
        let worker = thread::spawn(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                worker(
                    spec,
                    vocal_level.min(100),
                    profile,
                    command_rx,
                    worker_state,
                );
            }));

            let mut state = match exit_state.lock() {
                Ok(state) => state,
                Err(poisoned) => poisoned.into_inner(),
            };
            if outcome.is_err() {
                *state = RelayRuntimeState::Faulted("Windows audio relay worker panicked".into());
            } else if matches!(*state, RelayRuntimeState::Starting | RelayRuntimeState::Running) {
                *state = RelayRuntimeState::Stopped;
            }
        });
        Ok(Self {
            command_tx,
            state,
            worker: Some(worker),
        })
    }

    #[cfg(windows)]
    pub(crate) fn start_wasapi(spec: RelaySpec, vocal_level: u8) -> Result<Self, String> {
        Self::start_wasapi_with_profile(spec, vocal_level, ClassicSuppressionProfile::default())
    }

    #[cfg(windows)]
    pub(crate) fn start_wasapi_with_profile(
        spec: RelaySpec,
        vocal_level: u8,
        profile: ClassicSuppressionProfile,
    ) -> Result<Self, String> {
        Self::spawn_with_worker_profile(
            spec,
            vocal_level,
            profile,
            |spec, level, profile, commands, state| {
                let _ = crate::wasapi_relay::run_relay_worker_with_profile(
                    spec, level, profile, commands, state,
                );
            },
        )
    }

    pub(crate) fn state(&self) -> RelayRuntimeState {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_else(|_| RelayRuntimeState::Faulted("relay state lock is poisoned".into()))
    }

    pub(crate) fn set_vocal_level(&self, value: u8) -> Result<(), String> {
        self.command_tx
            .send(RelayCommand::SetVocalLevel(value.min(100)))
            .map_err(|_| "Windows audio relay worker is not running".to_string())
    }

    pub(crate) fn set_suppression_profile(
        &self,
        profile: ClassicSuppressionProfile,
    ) -> Result<(), String> {
        self.command_tx
            .send(RelayCommand::SetSuppressionProfile(profile))
            .map_err(|_| "Windows audio relay worker is not running".to_string())
    }

    pub(crate) fn stop(&mut self) -> Result<(), String> {
        let _ = self.command_tx.send(RelayCommand::Stop);
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| "Windows audio relay worker wrapper panicked".to_string())?;
        }
        if let Ok(mut state) = self.state.lock() {
            *state = RelayRuntimeState::Stopped;
        }
        Ok(())
    }
}

impl Drop for RelayHandle {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn running_state_is_required_for_ready() {
        assert!(!RelayRuntimeState::Starting.is_running());
        assert!(RelayRuntimeState::Running.is_running());
        assert!(!RelayRuntimeState::Faulted("device removed".into()).is_running());
    }

    #[test]
    fn route_identity_collision_is_rejected_before_spawn() {
        let spec = RelaySpec {
            source_endpoint_id: "same".into(),
            physical_output_endpoint_id: "same".into(),
        };
        assert!(RelayHandle::validate_spec(&spec).is_err());
    }

    #[test]
    fn distinct_route_is_accepted_before_spawn() {
        let spec = RelaySpec {
            source_endpoint_id: "cable".into(),
            physical_output_endpoint_id: "speakers".into(),
        };
        assert_eq!(RelayHandle::validate_spec(&spec), Ok(()));
    }

    #[test]
    fn fake_worker_can_reach_running_and_stop_cleanly() {
        let spec = RelaySpec {
            source_endpoint_id: "cable".into(),
            physical_output_endpoint_id: "speakers".into(),
        };
        let mut handle = RelayHandle::spawn_with_worker(
            spec,
            50,
            |_spec, _level, commands, state| {
                *state.lock().unwrap() = RelayRuntimeState::Running;
                if matches!(commands.recv().unwrap(), RelayCommand::Stop) {
                    *state.lock().unwrap() = RelayRuntimeState::Stopped;
                }
            },
        )
        .unwrap();

        handle.stop().unwrap();
        assert_eq!(handle.state(), RelayRuntimeState::Stopped);
    }

    #[test]
    fn profile_is_hot_switched_through_the_running_worker() {
        let spec = RelaySpec {
            source_endpoint_id: "cable".into(),
            physical_output_endpoint_id: "speakers".into(),
        };
        let observed = Arc::new(Mutex::new(Vec::new()));
        let worker_observed = Arc::clone(&observed);
        let mut handle = RelayHandle::spawn_with_worker_profile(
            spec,
            50,
            ClassicSuppressionProfile::MusicPreservation,
            move |_spec, _level, initial_profile, commands, state| {
                worker_observed.lock().unwrap().push(initial_profile);
                *state.lock().unwrap() = RelayRuntimeState::Running;
                loop {
                    match commands.recv().unwrap() {
                        RelayCommand::SetSuppressionProfile(profile) => {
                            worker_observed.lock().unwrap().push(profile);
                        }
                        RelayCommand::Stop => break,
                        RelayCommand::SetVocalLevel(_) => {}
                    }
                }
            },
        )
        .unwrap();

        handle
            .set_suppression_profile(ClassicSuppressionProfile::Balanced)
            .unwrap();
        handle.stop().unwrap();
        assert_eq!(
            *observed.lock().unwrap(),
            vec![
                ClassicSuppressionProfile::MusicPreservation,
                ClassicSuppressionProfile::Balanced,
            ]
        );
    }

    #[test]
    fn normal_worker_return_cannot_leave_backend_running() {
        let spec = RelaySpec {
            source_endpoint_id: "cable".into(),
            physical_output_endpoint_id: "speakers".into(),
        };
        let mut handle = RelayHandle::spawn_with_worker(
            spec,
            50,
            |_spec, _level, _commands, state| {
                *state.lock().unwrap() = RelayRuntimeState::Running;
            },
        )
        .unwrap();

        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if handle.state() == RelayRuntimeState::Stopped {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(handle.state(), RelayRuntimeState::Stopped);
        handle.stop().unwrap();
    }

    #[test]
    fn worker_panic_cannot_leave_backend_running() {
        let spec = RelaySpec {
            source_endpoint_id: "cable".into(),
            physical_output_endpoint_id: "speakers".into(),
        };
        let mut handle = RelayHandle::spawn_with_worker(
            spec,
            50,
            |_spec, _level, _commands, state| {
                *state.lock().unwrap() = RelayRuntimeState::Running;
                panic!("simulated relay panic");
            },
        )
        .unwrap();

        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if matches!(handle.state(), RelayRuntimeState::Faulted(_)) {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(matches!(handle.state(), RelayRuntimeState::Faulted(_)));
        handle.stop().unwrap();
    }
}
