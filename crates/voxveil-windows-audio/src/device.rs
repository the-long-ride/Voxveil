#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndpointDescriptor {
    pub id: String,
    pub name: String,
    pub interface_name: Option<String>,
    pub description: Option<String>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsInterceptionKind {
    Apo,
    VbCableRelay,
    VoxveilCableRelay,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowsAudioRoute {
    pub interception: Option<WindowsInterceptionKind>,
    pub source_endpoint_id: Option<String>,
    pub source_display_name: Option<String>,
    pub physical_output_endpoint_id: Option<String>,
    pub physical_output_display_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendProbe {
    pub readiness: RelayReadiness,
    pub route: WindowsAudioRoute,
    pub detail: Option<String>,
}

impl BackendProbe {
    pub fn unsupported() -> Self {
        Self {
            readiness: RelayReadiness::Unsupported,
            route: WindowsAudioRoute::default(),
            detail: None,
        }
    }
}

pub(crate) fn component_probe(
    control_available: bool,
    loaded_instances: u32,
    physical_output: Option<String>,
) -> BackendProbe {
    let physical_route = WindowsAudioRoute {
        physical_output_display_name: physical_output,
        ..WindowsAudioRoute::default()
    };

    if !control_available {
        return BackendProbe {
            readiness: RelayReadiness::ComponentRequired,
            route: physical_route,
            detail: Some(
                "Voxveil system-audio component is not installed beside the application".into(),
            ),
        };
    }

    if loaded_instances == 0 {
        return BackendProbe {
            readiness: RelayReadiness::ComponentRequired,
            route: physical_route,
            detail: Some(
                "VoxveilApo.dll is installed but AudioDG has not loaded it on the active render endpoint"
                    .into(),
            ),
        };
    }

    BackendProbe {
        readiness: RelayReadiness::Ready,
        route: WindowsAudioRoute {
            interception: Some(WindowsInterceptionKind::Apo),
            ..physical_route
        },
        detail: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_probe_carries_interception_route() {
        let probe = BackendProbe {
            readiness: RelayReadiness::Ready,
            route: WindowsAudioRoute {
                interception: Some(WindowsInterceptionKind::VbCableRelay),
                source_endpoint_id: Some("cable".into()),
                source_display_name: Some("CABLE Input (VB-Audio Virtual Cable)".into()),
                physical_output_endpoint_id: Some("speakers".into()),
                physical_output_display_name: Some("Speakers".into()),
            },
            detail: None,
        };
        assert_eq!(
            probe.route.interception,
            Some(WindowsInterceptionKind::VbCableRelay)
        );
        assert_eq!(
            probe.route.physical_output_endpoint_id.as_deref(),
            Some("speakers")
        );
    }

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
        assert_eq!(
            probe.route.physical_output_display_name.as_deref(),
            Some("Speakers")
        );
        assert_eq!(probe.route.interception, Some(WindowsInterceptionKind::Apo));
    }

    #[test]
    fn virtual_endpoint_name_no_longer_changes_readiness() {
        let probe = component_probe(true, 0, Some("SYSVAD (with APO Extensions)".into()));
        assert_ne!(probe.readiness, RelayReadiness::Ready);
    }
}
