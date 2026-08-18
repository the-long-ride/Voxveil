use crate::{BackendProbe, RelayReadiness};

pub struct ApoBackend;

impl ApoBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn probe(&self) -> BackendProbe {
        BackendProbe {
            readiness: RelayReadiness::ComponentRequired,
            physical_output: None,
            detail: Some(
                "Voxveil APO component is not installed or not attached to the active output"
                    .into(),
            ),
        }
    }

    pub fn set_enabled(
        &mut self,
        enabled: bool,
        _vocal_level: u8,
    ) -> Result<BackendProbe, String> {
        let probe = self.probe();
        if enabled {
            Err(probe
                .detail
                .clone()
                .unwrap_or_else(|| "Voxveil APO is unavailable".into()))
        } else {
            Ok(probe)
        }
    }

    pub fn set_vocal_level(&self, _value: u8) {}
}

impl Default for ApoBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uninstalled_apo_never_claims_ready() {
        let backend = ApoBackend::new();
        let probe = backend.probe();
        assert_eq!(probe.readiness, RelayReadiness::ComponentRequired);
        assert!(probe.detail.is_some());
    }

    #[test]
    fn unavailable_apo_refuses_enable() {
        let mut backend = ApoBackend::new();
        assert!(backend.set_enabled(true, 50).is_err());
        assert!(backend.set_enabled(false, 50).is_ok());
    }
}
