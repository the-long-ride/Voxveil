use crate::discovery::{SystemAudioEndpointStatus, classify_binding};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeBindingKind {
    Unique,
    Ambiguous,
    None,
}

pub(crate) fn classify_runtime_binding(
    runtime_binding: RuntimeBindingKind,
    fallback_pnp_resolved: bool,
    topology_references: &[String],
    package_available: bool,
) -> SystemAudioEndpointStatus {
    match runtime_binding {
        RuntimeBindingKind::Ambiguous => SystemAudioEndpointStatus::Ambiguous,
        RuntimeBindingKind::Unique => match topology_references.len() {
            0 => SystemAudioEndpointStatus::ComponentRequired,
            1 if package_available => SystemAudioEndpointStatus::Installable,
            1 => SystemAudioEndpointStatus::ComponentRequired,
            _ => SystemAudioEndpointStatus::Ambiguous,
        },
        RuntimeBindingKind::None => classify_binding(
            fallback_pnp_resolved,
            topology_references,
            package_available,
        ),
    }
}

pub(crate) fn fallback_device_matches_runtime(
    runtime_device_id: Option<&str>,
    fallback_device_id: Option<&str>,
) -> bool {
    match runtime_device_id {
        None => true,
        Some(runtime) => fallback_device_id
            .is_some_and(|fallback| runtime.eq_ignore_ascii_case(fallback)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_runtime_binding_without_reference_requires_component() {
        assert_eq!(
            classify_runtime_binding(RuntimeBindingKind::Unique, false, &[], false),
            SystemAudioEndpointStatus::ComponentRequired
        );
    }

    #[test]
    fn unique_runtime_binding_with_matching_package_is_installable() {
        assert_eq!(
            classify_runtime_binding(
                RuntimeBindingKind::Unique,
                true,
                &["PrimaryLineOutTopo".into()],
                true,
            ),
            SystemAudioEndpointStatus::Installable
        );
    }

    #[test]
    fn runtime_ambiguity_cannot_be_overridden_by_inf_fallback() {
        assert_eq!(
            classify_runtime_binding(
                RuntimeBindingKind::Ambiguous,
                true,
                &["PrimaryLineOutTopo".into()],
                true,
            ),
            SystemAudioEndpointStatus::Ambiguous
        );
    }

    #[test]
    fn missing_runtime_binding_uses_fallback_classification() {
        assert_eq!(
            classify_runtime_binding(
                RuntimeBindingKind::None,
                true,
                &["PrimaryLineOutTopo".into()],
                false,
            ),
            SystemAudioEndpointStatus::ComponentRequired
        );
    }

    #[test]
    fn runtime_selected_device_rejects_fallback_metadata_from_another_device() {
        assert!(!fallback_device_matches_runtime(
            Some("HDAUDIO\\FUNC_01"),
            Some("HDAUDIO\\OTHER"),
        ));
        assert!(fallback_device_matches_runtime(
            Some("HDAUDIO\\FUNC_01"),
            Some("hdaudio\\func_01"),
        ));
    }
}
