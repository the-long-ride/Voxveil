#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndpointDescriptor {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayReadiness {
    Ready,
    ComponentRequired,
    RoutingRequired,
    Faulted,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendProbe {
    pub readiness: RelayReadiness,
    pub physical_output: Option<String>,
    pub detail: Option<String>,
}

impl BackendProbe {
    pub fn unsupported() -> Self {
        Self {
            readiness: RelayReadiness::Unsupported,
            physical_output: None,
            detail: None,
        }
    }
}

pub(crate) fn component_probe(
    control_available: bool,
    loaded_instances: u32,
    physical_output: Option<String>,
) -> BackendProbe {
    if !control_available {
        return BackendProbe {
            readiness: RelayReadiness::ComponentRequired,
            physical_output,
            detail: Some(
                "Voxveil system-audio component is not installed beside the application".into(),
            ),
        };
    }

    if loaded_instances == 0 {
        return BackendProbe {
            readiness: RelayReadiness::ComponentRequired,
            physical_output,
            detail: Some(
                "VoxveilApo.dll is installed but AudioDG has not loaded it on the active render endpoint"
                    .into(),
            ),
        };
    }

    BackendProbe {
        readiness: RelayReadiness::Ready,
        physical_output,
        detail: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_control_component_is_not_ready() {
        let probe = component_probe(false, 0, Some("Speakers".into()));
        assert_eq!(probe.readiness, RelayReadiness::ComponentRequired);
    }

    #[test]
    fn installed_but_not_loaded_apo_is_not_ready() {
        let probe = component_probe(true, 0, Some("Speakers".into()));
        assert_eq!(probe.readiness, RelayReadiness::ComponentRequired);
        assert!(probe.detail.unwrap().contains("AudioDG has not loaded"));
    }

    #[test]
    fn loaded_apo_is_ready() {
        let probe = component_probe(true, 1, Some("Speakers".into()));
        assert_eq!(probe.readiness, RelayReadiness::Ready);
        assert_eq!(probe.physical_output.as_deref(), Some("Speakers"));
    }

    #[test]
    fn virtual_endpoint_name_no_longer_changes_readiness() {
        let probe = component_probe(true, 0, Some("SYSVAD (with APO Extensions)".into()));
        assert_ne!(probe.readiness, RelayReadiness::Ready);
    }
}
