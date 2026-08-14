use std::sync::Mutex;

use voxveil_types::ProcessingBackendStatus;

#[derive(Clone, Debug)]
pub struct BackendSnapshot {
    pub status: ProcessingBackendStatus,
    pub physical_output: Option<String>,
    pub per_app_available: bool,
}

pub struct ProcessingController {
    #[cfg(target_os = "windows")]
    backend: Mutex<voxveil_windows_audio::WindowsAudioBackend>,
}

impl Default for ProcessingController {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            backend: Mutex::new(voxveil_windows_audio::WindowsAudioBackend::new()),
        }
    }
}

impl ProcessingController {
    pub fn snapshot(&self) -> BackendSnapshot {
        #[cfg(target_os = "windows")]
        {
            let Ok(mut backend) = self.backend.lock() else {
                return faulted_snapshot();
            };
            return from_windows_probe(backend.probe());
        }
        #[cfg(not(target_os = "windows"))]
        BackendSnapshot {
            status: super::processing_backend_status(),
            physical_output: None,
            per_app_available: false,
        }
    }

    pub fn set_enabled(&self, enabled: bool, vocal_level: u8) -> Result<BackendSnapshot, String> {
        #[cfg(target_os = "windows")]
        {
            let mut backend = self
                .backend
                .lock()
                .map_err(|_| "Windows audio backend lock is poisoned".to_string())?;
            return backend
                .set_enabled(enabled, vocal_level)
                .map(from_windows_probe);
        }
        #[cfg(not(target_os = "windows"))]
        {
            if enabled {
                Err("system-audio processing is unavailable on this platform build".into())
            } else {
                Ok(self.snapshot())
            }
        }
    }

    pub fn set_vocal_level(&self, value: u8) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            let backend = self
                .backend
                .lock()
                .map_err(|_| "Windows audio backend lock is poisoned".to_string())?;
            backend.set_vocal_level(value);
        }
        Ok(())
    }

    pub fn physical_outputs(&self) -> Vec<String> {
        #[cfg(target_os = "windows")]
        {
            return self
                .backend
                .lock()
                .map(|backend| backend.physical_outputs())
                .unwrap_or_default();
        }
        #[cfg(not(target_os = "windows"))]
        Vec::new()
    }
}

#[cfg(target_os = "windows")]
fn from_windows_probe(probe: voxveil_windows_audio::BackendProbe) -> BackendSnapshot {
    use voxveil_windows_audio::RelayReadiness;
    let status = match probe.readiness {
        RelayReadiness::Ready => ProcessingBackendStatus::Ready,
        RelayReadiness::ComponentRequired => ProcessingBackendStatus::ComponentRequired,
        RelayReadiness::RoutingRequired => ProcessingBackendStatus::RoutingRequired,
        RelayReadiness::Faulted => ProcessingBackendStatus::Faulted,
        RelayReadiness::Unsupported => ProcessingBackendStatus::Unsupported,
    };
    BackendSnapshot {
        status,
        physical_output: probe.physical_output,
        per_app_available: false,
    }
}

fn faulted_snapshot() -> BackendSnapshot {
    BackendSnapshot {
        status: ProcessingBackendStatus::Faulted,
        physical_output: None,
        per_app_available: false,
    }
}
