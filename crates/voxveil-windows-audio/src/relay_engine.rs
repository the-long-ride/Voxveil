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
    #[cfg(test)]
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
        validate_distinct_route(&spec.source_endpoint_id, &spec.physical_output_endpoint_id)
            .map_err(str::to_string)
    }

    #[cfg(test)]
    pub(crate) fn spawn_with_worker<F>(
        spec: RelaySpec,
        vocal_level: u8,
        worker: F,
    ) -> Result<Self, String>
    where
        F: FnOnce(RelaySpec, u8, Receiver<RelayCommand>, Arc<Mutex<RelayRuntimeState>>)
            + Send
            + 'static,
    {
        Self::spawn_with_worker_profile(
            spec,
            vocal_level,
            ClassicSuppressionProfile::default(),
            move |spec, level, _profile, commands, state| worker(spec, level, commands, state),
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
                worker(spec, vocal_level.min(100), profile, command_rx, worker_state);
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
#[path = "relay_engine_tests.rs"]
mod tests;
