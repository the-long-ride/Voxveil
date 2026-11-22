use super::*;

impl WindowsAudioBackend {
    pub fn probe(&mut self) -> BackendProbe {
        let endpoints = match enumerate_render_blocking() {
            Ok(endpoints) => endpoints,
            Err(error) => {
                self.disable_processing_best_effort();
                return fault_probe(None, error);
            }
        };
        let (loaded_instances, apo_covers_default) = match query_apo_coverage(&endpoints) {
            Ok(value) => value,
            Err(error) => {
                self.disable_processing_best_effort();
                return fault_probe(None, format!("failed to query Voxveil APO status: {error}"));
            }
        };
        let selected = self
            .interception_policy
            .allows_virtual_relay()
            .then(|| select_relay_source(&endpoints))
            .flatten();
        let preferred_physical_output_id =
            if self.interception_policy == WindowsInterceptionPolicy::PhysicalApoOnly {
                None
            } else {
                self.preferred_physical_output_id.as_deref()
            };
        let physical = select_relay_output(&endpoints, selected, preferred_physical_output_id);

        if !self.interception_policy.allows_virtual_relay() {
            if let Some(mut relay) = self.relay.take() {
                if let Err(error) = relay.stop() {
                    self.disable_processing_best_effort();
                    return fault_probe(
                        physical.map(|endpoint| endpoint.name.clone()),
                        format!(
                            "failed to stop Windows audio relay for the selected route: {error}"
                        ),
                    );
                }
                self.enabled = false;
            }
        }

        if !self.interception_policy.allows_physical_apo() {
            if loaded_instances > 0 {
                if let Err(error) = set_apo_enabled(false) {
                    self.disable_processing_best_effort();
                    return fault_probe(
                        physical.map(|endpoint| endpoint.name.clone()),
                        format!(
                            "failed to disable Voxveil APO while local file playback is selected: {error}"
                        ),
                    );
                }
            }
            self.enabled = false;
            return decide_backend_with_policy(
                self.interception_policy,
                false,
                None,
                physical,
                None,
            );
        }

        if apo_covers_default {
            let relay_was_active = self.relay.is_some();
            if let Some(mut relay) = self.relay.take() {
                if let Err(error) = relay.stop() {
                    self.disable_processing_best_effort();
                    return fault_probe(
                        physical.map(|endpoint| endpoint.name.clone()),
                        format!("failed to stop Windows audio relay during APO handoff: {error}"),
                    );
                }
            }
            if should_sync_apo_after_relay(apo_covers_default, relay_was_active, self.enabled) {
                if let Err(error) =
                    sync_apo_control(self.vocal_level, self.classic_suppression_profile, true)
                {
                    self.disable_processing_best_effort();
                    return fault_probe(
                        physical.map(|endpoint| endpoint.name.clone()),
                        format!(
                            "failed to transfer active processing state to Voxveil APO: {error}"
                        ),
                    );
                }
            }
            return decide_backend(true, selected, physical, None);
        }

        if !self.interception_policy.allows_virtual_relay() {
            if loaded_instances > 0 {
                if let Err(error) = set_apo_enabled(false) {
                    self.disable_processing_best_effort();
                    return fault_probe(
                        physical.map(|endpoint| endpoint.name.clone()),
                        format!(
                            "failed to disable a Voxveil APO that does not cover the active physical endpoint: {error}"
                        ),
                    );
                }
            }
            self.enabled = false;
            return decide_backend_with_policy(
                self.interception_policy,
                false,
                None,
                physical,
                None,
            );
        }

        if loaded_instances > 0 && self.enabled {
            if let Err(error) = set_apo_enabled(false) {
                self.disable_processing_best_effort();
                return fault_probe(
                    physical.map(|endpoint| endpoint.name.clone()),
                    format!("failed to disable a Voxveil APO loaded on another endpoint: {error}"),
                );
            }
        }

        if self.relay.as_ref().is_some_and(|relay| {
            !selected.zip(physical).is_some_and(|((_, source), output)| {
                relay.matches_route(&source.id, &output.id)
            })
        }) {
            if let Some(mut relay) = self.relay.take() {
                self.enabled = false;
                if let Err(error) = relay.stop() {
                    return fault_probe(None, error);
                }
            }
        }
        let relay_state = self.relay.as_ref().map(RelayHandle::state);
        let decision = decide_backend(false, selected, physical, relay_state.as_ref());
        if should_fail_closed_after_probe(decision.readiness) {
            self.disable_processing_best_effort();
        }
        decision
    }
}
