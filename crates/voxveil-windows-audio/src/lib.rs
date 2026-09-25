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
mod playback;
#[cfg(windows)]
mod wasapi_relay;
#[cfg(windows)]
#[allow(unsafe_code)]
mod topology;

#[cfg(not(windows))]
use voxveil_types::{ClassicSuppressionProfile, WindowsInterceptionPolicy};

pub use device::{
    BackendProbe, EndpointDescriptor, RelayReadiness, WindowsAudioRoute, WindowsInterceptionKind,
};
pub use discovery::{SystemAudioEndpoint, SystemAudioEndpointStatus};
pub use sample::process_f32le_stereo;

#[cfg(windows)]
pub use relay::WindowsAudioBackend;
#[cfg(windows)]
pub use playback::{OwnedPlayback, OwnedPlaybackSnapshot, PlaybackStatus};
#[cfg(not(windows))]
pub use playback_stub::{OwnedPlayback, OwnedPlaybackSnapshot, PlaybackStatus};

#[cfg(not(windows))]
mod playback_stub {
    use std::path::PathBuf;
    use voxveil_types::{AudioPlaybackSnapshot, AudioPlaybackStatus, ClassicSuppressionProfile};

    pub type OwnedPlaybackSnapshot = AudioPlaybackSnapshot;
    pub type PlaybackStatus = AudioPlaybackStatus;

    #[derive(Default)]
    pub struct OwnedPlayback;

    impl OwnedPlayback {
        pub fn open(
            &mut self,
            _path: PathBuf,
            _endpoint: super::EndpointDescriptor,
            _vocal_level: u8,
            _profile: ClassicSuppressionProfile,
        ) -> Result<OwnedPlaybackSnapshot, String> {
            Err("owned file playback is unavailable on this platform".into())
        }
        pub fn snapshot(&self) -> OwnedPlaybackSnapshot { OwnedPlaybackSnapshot::default() }
        pub fn pause(&self) -> Result<(), String> { Err("owned file playback is unavailable on this platform".into()) }
        pub fn resume(&self) -> Result<(), String> { Err("owned file playback is unavailable on this platform".into()) }
        pub fn seek(&self, _frame: u64) -> Result<(), String> { Err("owned file playback is unavailable on this platform".into()) }
        pub fn set_vocal_level(&self, _value: u8) -> Result<(), String> { Ok(()) }
        pub fn set_profile(&self, _profile: ClassicSuppressionProfile) -> Result<(), String> { Ok(()) }
        pub fn stop(&mut self) -> Result<(), String> { Ok(()) }
    }
}
#[cfg(windows)]
pub fn windows_system_directory() -> Result<std::path::PathBuf, String> {
    topology::windows_system_directory()
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
        Err("Windows audio relay is unavailable on this platform".into())
    }
    pub fn set_vocal_level(&self, _value: u8) -> Result<(), String> {
        Ok(())
    }
    pub fn set_classic_suppression_profile(
        &mut self,
        _profile: ClassicSuppressionProfile,
    ) -> Result<(), String> {
        Ok(())
    }
    pub fn set_interception_policy(
        &mut self,
        _policy: WindowsInterceptionPolicy,
    ) -> BackendProbe {
        BackendProbe::unsupported()
    }
    pub fn physical_outputs(&self) -> Result<Vec<EndpointDescriptor>, String> {
        Ok(Vec::new())
    }
    pub fn system_audio_endpoints(&self) -> Result<Vec<SystemAudioEndpoint>, String> {
        Ok(Vec::new())
    }
}
