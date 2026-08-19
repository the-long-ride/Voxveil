#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemAudioEndpointStatus {
    Ready,
    Installable,
    ComponentRequired,
    Ambiguous,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemAudioEndpoint {
    pub endpoint_id: String,
    pub display_name: String,
    pub adapter_name: Option<String>,
    pub is_default: bool,
    pub pnp_instance_id: Option<String>,
    pub hardware_ids: Vec<String>,
    pub driver_inf: Option<String>,
    pub topology_reference: Option<String>,
    pub status: SystemAudioEndpointStatus,
    pub detail: Option<String>,
}

pub(crate) fn classify_binding(
    _pnp_resolved: bool,
    _topology_candidates: &[String],
    _package_available: bool,
) -> SystemAudioEndpointStatus {
    SystemAudioEndpointStatus::Unsupported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_topology_is_installable_when_package_is_available() {
        assert_eq!(
            classify_binding(true, &["Topology".into()], true),
            SystemAudioEndpointStatus::Installable
        );
    }

    #[test]
    fn multiple_topology_candidates_fail_closed() {
        assert_eq!(
            classify_binding(
                true,
                &["Topology".into(), "HeadphoneTopology".into()],
                true
            ),
            SystemAudioEndpointStatus::Ambiguous
        );
    }

    #[test]
    fn missing_pnp_identity_is_unsupported() {
        assert_eq!(
            classify_binding(false, &["Topology".into()], true),
            SystemAudioEndpointStatus::Unsupported
        );
    }

    #[test]
    fn resolved_binding_without_installable_package_requires_component() {
        assert_eq!(
            classify_binding(true, &["Topology".into()], false),
            SystemAudioEndpointStatus::ComponentRequired
        );
    }
}
