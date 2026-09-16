use std::sync::Mutex;

use voxveil_types::{ClassicSuppressionProfile, ProcessingBackendStatus};

#[derive(Clone, Debug)]
pub struct BackendSnapshot {
    pub status: ProcessingBackendStatus,
    pub backend_kind: Option<String>,
    pub physical_output: Option<String>,
    pub physical_output_endpoint_id: Option<String>,
    pub per_app_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalOutput {
    pub endpoint_id: String,
    pub display_name: String,
    pub is_default: bool,
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
            backend_kind: None,
            physical_output: None,
            physical_output_endpoint_id: None,
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
            let mut backend = self
                .backend
                .lock()
                .map_err(|_| "Windows audio backend lock is poisoned".to_string())?;
            return backend.set_vocal_level(value);
        }
        #[cfg(not(target_os = "windows"))]
        Ok(())
    }

    pub fn set_classic_suppression_profile(
        &self,
        profile: ClassicSuppressionProfile,
    ) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            let mut backend = self
                .backend
                .lock()
                .map_err(|_| "Windows audio backend lock is poisoned".to_string())?;
            return backend.set_classic_suppression_profile(profile);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = profile;
            Ok(())
        }
    }

    pub fn set_physical_output(
        &self,
        endpoint_id: Option<String>,
    ) -> Result<BackendSnapshot, String> {
        #[cfg(target_os = "windows")]
        {
            let mut backend = self
                .backend
                .lock()
                .map_err(|_| "Windows audio backend lock is poisoned".to_string())?;
            return backend
                .set_physical_output(endpoint_id)
                .map(from_windows_probe);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = endpoint_id;
            Err("physical Windows audio routing is unavailable on this platform".into())
        }
    }

    pub fn physical_outputs(&self) -> Vec<PhysicalOutput> {
        #[cfg(target_os = "windows")]
        {
            return self
                .backend
                .lock()
                .map(|backend| {
                    backend
                        .physical_outputs()
                        .into_iter()
                        .map(|output| PhysicalOutput {
                            endpoint_id: output.id,
                            display_name: output.name,
                            is_default: output.is_default,
                        })
                        .collect()
                })
                .unwrap_or_default();
        }
        #[cfg(not(target_os = "windows"))]
        Vec::new()
    }

    #[cfg(target_os = "windows")]
    pub fn system_audio_endpoints(
        &self,
    ) -> Result<Vec<voxveil_windows_audio::SystemAudioEndpoint>, String> {
        self.backend
            .lock()
            .map_err(|_| "Windows audio backend lock is poisoned".to_string())?
            .system_audio_endpoints()
    }
}

#[cfg(target_os = "windows")]
fn interception_name(kind: voxveil_windows_audio::WindowsInterceptionKind) -> &'static str {
    use voxveil_windows_audio::WindowsInterceptionKind;
    match kind {
        WindowsInterceptionKind::Apo => "apo",
        WindowsInterceptionKind::VbCableRelay => "vb-cable-relay",
        WindowsInterceptionKind::VoxveilCableRelay => "voxveil-cable-relay",
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
    let route = probe.route;
    BackendSnapshot {
        status,
        backend_kind: route.interception.map(interception_name).map(str::to_string),
        physical_output: route.physical_output_display_name,
        physical_output_endpoint_id: route.physical_output_endpoint_id,
        per_app_available: false,
    }
}

fn faulted_snapshot() -> BackendSnapshot {
    BackendSnapshot {
        status: ProcessingBackendStatus::Faulted,
        backend_kind: None,
        physical_output: None,
        physical_output_endpoint_id: None,
        per_app_available: false,
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use voxveil_windows_audio::WindowsInterceptionKind;

    #[test]
    fn interception_names_are_stable_for_ui_contract() {
        assert_eq!(interception_name(WindowsInterceptionKind::Apo), "apo");
        assert_eq!(
            interception_name(WindowsInterceptionKind::VbCableRelay),
            "vb-cable-relay"
        );
        assert_eq!(
            interception_name(WindowsInterceptionKind::VoxveilCableRelay),
            "voxveil-cable-relay"
        );
    }
}
