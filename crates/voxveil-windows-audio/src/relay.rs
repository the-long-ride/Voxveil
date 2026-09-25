use std::time::{Duration, Instant};

use voxveil_types::{ClassicSuppressionProfile, WindowsInterceptionPolicy};

use crate::device::{BackendProbe, EndpointDescriptor};
use crate::discovery::{SystemAudioEndpoint, SystemAudioEndpointStatus, enrich_endpoints};
use crate::relay_engine::{RelayHandle, RelayRuntimeState, RelaySpec};
use crate::route::select_physical_output;
use crate::virtual_endpoint::{
    VirtualEndpointKind, classify_virtual_endpoint, find_preferred_virtual_endpoint,
};

#[path = "relay_control.rs"]
mod control;
#[path = "relay_enumeration.rs"]
mod enumeration;
#[path = "relay_probe.rs"]
mod probe;
#[path = "relay_support.rs"]
mod support;

use control::{
    control_executable, control_executable_for_installed_apo, run_control, system_audio_directory,
};
use enumeration::enumerate_render_blocking;
use support::{
    decide_backend, decide_backend_with_policy, fault_probe, profile_control_value,
    query_apo_coverage, set_apo_enabled, should_fail_closed_after_probe,
    should_sync_apo_after_relay, sync_apo_control,
};

pub struct WindowsAudioBackend {
    enabled: bool,
    vocal_level: u8,
    classic_suppression_profile: ClassicSuppressionProfile,
    relay: Option<RelayHandle>,
    preferred_physical_output_id: Option<String>,
    interception_policy: WindowsInterceptionPolicy,
}

impl WindowsAudioBackend {
    pub fn new() -> Self {
        Self {
            enabled: false,
            vocal_level: 100,
            classic_suppression_profile: ClassicSuppressionProfile::default(),
            relay: None,
            preferred_physical_output_id: None,
            interception_policy: WindowsInterceptionPolicy::Automatic,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool, vocal_level: u8) -> Result<BackendProbe, String> {
        self.vocal_level = vocal_level.min(100);
        if !enabled {
            self.enabled = false;
            if let Some(mut relay) = self.relay.take() {
                relay.stop()?;
            }
            if let Some(control) = control_executable_for_installed_apo()? {
                run_control(&control, &["enabled", "0"])?;
            }
            return Ok(self.probe());
        }

        if !self.interception_policy.allows_physical_apo() {
            return Err(
                "system-wide audio processing is disabled while local file playback is selected"
                    .into(),
            );
        }

        let endpoints = enumerate_render_blocking()?;
        let (loaded_instances, apo_covers_default) = match query_apo_coverage(&endpoints) {
            Ok(value) => value,
            Err(error) => {
                self.disable_processing_best_effort();
                return Err(format!("failed to query Voxveil APO status: {error}"));
            }
        };
        if apo_covers_default {
            if let Some(mut relay) = self.relay.take() {
                relay.stop()?;
            }
            sync_apo_control(self.vocal_level, self.classic_suppression_profile, true)?;
            self.enabled = true;
            return Ok(self.probe());
        }
        if loaded_instances > 0 {
            set_apo_enabled(false).map_err(|error| {
                format!("failed to disable a Voxveil APO loaded on another endpoint: {error}")
            })?;
        }

        if !self.interception_policy.allows_virtual_relay() {
            self.enabled = false;
            return Ok(self.probe());
        }

        let (virtual_kind, source) = find_preferred_virtual_endpoint(&endpoints).ok_or_else(|| {
            "Install the standard VB-CABLE virtual audio device or a verified Voxveil virtual audio driver to enable system-wide processing".to_string()
        })?;
        if !source.is_default {
            return Err(match virtual_kind {
                VirtualEndpointKind::VoxveilCable => {
                    "Set Voxveil Input as the Windows default output before enabling Voxveil".into()
                }
                VirtualEndpointKind::VbCable => {
                    "Set CABLE Input as the Windows default output before enabling Voxveil".into()
                }
            });
        }
        let physical = select_physical_output(
            &endpoints,
            &source.id,
            self.preferred_physical_output_id.as_deref(),
        )
        .ok_or_else(|| {
            "No safe physical playback endpoint is available for the relay".to_string()
        })?;

        if let Some(mut relay) = self.relay.take() {
            relay.stop()?;
        }
        let spec = RelaySpec {
            source_endpoint_id: source.id.clone(),
            physical_output_endpoint_id: physical.id.clone(),
        };
        let relay = RelayHandle::start_wasapi_with_profile(
            spec,
            self.vocal_level,
            self.classic_suppression_profile,
        )?;
        self.relay = Some(relay);

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let state = self
                .relay
                .as_ref()
                .map(RelayHandle::state)
                .unwrap_or(RelayRuntimeState::Stopped);
            match state {
                RelayRuntimeState::Running => {
                    self.enabled = true;
                    return Ok(self.probe());
                }
                RelayRuntimeState::Faulted(error) => {
                    self.relay.take();
                    self.enabled = false;
                    return Err(error);
                }
                RelayRuntimeState::Stopped => {
                    self.relay.take();
                    self.enabled = false;
                    return Err("Windows audio relay stopped during startup".into());
                }
                RelayRuntimeState::Starting if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                RelayRuntimeState::Starting => {
                    if let Some(mut relay) = self.relay.take() {
                        let _ = relay.stop();
                    }
                    self.enabled = false;
                    return Err("Windows audio relay did not become ready within 2 seconds".into());
                }
            }
        }
    }

    pub fn set_vocal_level(&mut self, value: u8) -> Result<(), String> {
        let vocal_level = value.min(100);
        if let Some(relay) = &self.relay {
            relay.set_vocal_level(vocal_level)?;
        } else if let Some(control) = control_executable_for_installed_apo()? {
            let percent = vocal_level.to_string();
            run_control(&control, &["vocal", percent.as_str()])?;
        }
        self.vocal_level = vocal_level;
        Ok(())
    }

    pub fn set_classic_suppression_profile(
        &mut self,
        profile: ClassicSuppressionProfile,
    ) -> Result<(), String> {
        if let Some(relay) = &self.relay {
            relay.set_suppression_profile(profile)?;
        } else if let Some(control) = control_executable_for_installed_apo()? {
            run_control(&control, &["profile", profile_control_value(profile)])?;
        }
        self.classic_suppression_profile = profile;
        Ok(())
    }

    pub fn set_physical_output(
        &mut self,
        endpoint_id: Option<String>,
    ) -> Result<BackendProbe, String> {
        if let Some(endpoint_id) = endpoint_id.as_deref() {
            let endpoints = enumerate_render_blocking()?;
            let endpoint = endpoints
                .iter()
                .find(|endpoint| endpoint.id == endpoint_id)
                .ok_or_else(|| {
                    "The selected physical playback endpoint is no longer available".to_string()
                })?;
            if classify_virtual_endpoint(endpoint).is_some() {
                return Err(
                    "A virtual interception endpoint cannot be used as physical output".into(),
                );
            }
        }
        if let Some(mut relay) = self.relay.take() {
            self.enabled = false;
            relay.stop()?;
        }
        self.preferred_physical_output_id = endpoint_id;
        Ok(self.probe())
    }

    pub fn set_interception_policy(&mut self, policy: WindowsInterceptionPolicy) -> BackendProbe {
        self.interception_policy = policy;
        self.probe()
    }

    pub fn physical_outputs(&self) -> Result<Vec<EndpointDescriptor>, String> {
        Ok(enumerate_render_blocking()?
            .into_iter()
            .filter(|item| classify_virtual_endpoint(item).is_none())
            .collect())
    }

    pub fn system_audio_endpoints(&self) -> Result<Vec<SystemAudioEndpoint>, String> {
        let endpoints = enumerate_render_blocking()?;
        let (_, apo_covers_default) = query_apo_coverage(&endpoints)?;
        let directory = system_audio_directory();
        let helper = directory.join("discover-system-audio-endpoints.ps1");
        let mut enriched = enrich_endpoints(endpoints, &helper, &directory)?;
        if apo_covers_default {
            if let Some(active) = enriched.iter_mut().find(|endpoint| endpoint.is_default) {
                active.status = SystemAudioEndpointStatus::Ready;
                active.detail = None;
            }
        }
        Ok(enriched)
    }

    fn disable_processing_best_effort(&mut self) {
        self.enabled = false;
        if let Some(mut relay) = self.relay.take() {
            let _ = relay.stop();
        }
        if let Some(control) = control_executable() {
            let _ = run_control(&control, &["enabled", "0"]);
        }
    }
}

#[cfg(test)]
#[path = "relay_tests.rs"]
mod tests;
