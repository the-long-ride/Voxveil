#![deny(unsafe_code)]

mod apo_route;
mod binding;
mod device;
mod device_interfaces;
mod discovery;
mod relay_engine;
mod route;
mod sample;
mod virtual_endpoint;

#[cfg(windows)]
mod relay;
#[cfg(windows)]
mod wasapi_relay;
#[cfg(windows)]
#[allow(unsafe_code)]
mod topology;

use voxveil_types::ClassicSuppressionProfile;

pub use device::{
    BackendProbe, EndpointDescriptor, RelayReadiness, WindowsAudioRoute, WindowsInterceptionKind,
};
pub use discovery::{SystemAudioEndpoint, SystemAudioEndpointStatus};
pub use sample::process_f32le_stereo;

#[cfg(windows)]
pub use relay::WindowsAudioBackend;

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
        Err("Windows audio relay is unavailable on this platform".into())
    }
    pub fn set_vocal_level(&self, _value: u8) {}
    pub fn set_classic_suppression_profile(
        &mut self,
        _profile: ClassicSuppressionProfile,
    ) -> Result<(), String> {
        Ok(())
    }
    pub fn physical_outputs(&self) -> Vec<String> {
        Vec::new()
    }
    pub fn system_audio_endpoints(&self) -> Result<Vec<SystemAudioEndpoint>, String> {
        Ok(Vec::new())
    }
}
