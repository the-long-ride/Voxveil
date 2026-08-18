#![forbid(unsafe_code)]

mod apo;
mod backend;
mod device;
mod sample;

#[cfg(windows)]
mod relay;

pub use backend::WindowsBackendKind;
pub use device::{BackendProbe, EndpointDescriptor, RelayReadiness};
pub use sample::process_f32le_stereo;

#[cfg(windows)]
pub struct WindowsAudioBackend {
    apo: apo::ApoBackend,
    relay: relay::WindowsAudioBackend,
    active: WindowsBackendKind,
}

#[cfg(windows)]
impl WindowsAudioBackend {
    pub fn new() -> Self {
        Self {
            apo: apo::ApoBackend::new(),
            relay: relay::WindowsAudioBackend::new(),
            active: WindowsBackendKind::Relay,
        }
    }

    pub fn probe(&mut self) -> BackendProbe {
        let apo_probe = self.apo.probe();
        let relay_probe = self.relay.probe();
        self.active = backend::select_backend(&apo_probe, &relay_probe);
        match self.active {
            WindowsBackendKind::Apo => apo_probe,
            WindowsBackendKind::Relay => relay_probe,
        }
    }

    pub fn set_enabled(
        &mut self,
        enabled: bool,
        vocal_level: u8,
    ) -> Result<BackendProbe, String> {
        let apo_probe = self.apo.probe();
        if apo_probe.readiness == RelayReadiness::Ready {
            self.active = WindowsBackendKind::Apo;
            self.apo.set_enabled(enabled, vocal_level)
        } else {
            self.active = WindowsBackendKind::Relay;
            self.relay.set_enabled(enabled, vocal_level)
        }
    }

    pub fn set_vocal_level(&self, value: u8) {
        match self.active {
            WindowsBackendKind::Apo => self.apo.set_vocal_level(value),
            WindowsBackendKind::Relay => self.relay.set_vocal_level(value),
        }
    }

    pub fn physical_outputs(&self) -> Vec<String> {
        self.relay.physical_outputs()
    }

    pub fn active_kind(&self) -> WindowsBackendKind {
        self.active
    }
}

#[cfg(windows)]
impl Default for WindowsAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(windows))]
pub struct WindowsAudioBackend;

#[cfg(not(windows))]
impl WindowsAudioBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn probe(&self) -> BackendProbe {
        BackendProbe::unsupported()
    }

    pub fn set_enabled(
        &mut self,
        _enabled: bool,
        _vocal_level: u8,
    ) -> Result<BackendProbe, String> {
        Err("Windows audio processing is unavailable on this platform".into())
    }

    pub fn set_vocal_level(&self, _value: u8) {}

    pub fn physical_outputs(&self) -> Vec<String> {
        Vec::new()
    }

    pub fn active_kind(&self) -> WindowsBackendKind {
        WindowsBackendKind::Relay
    }
}

#[cfg(not(windows))]
impl Default for WindowsAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}
