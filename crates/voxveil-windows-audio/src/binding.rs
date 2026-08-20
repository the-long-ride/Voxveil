use crate::discovery::{SystemAudioEndpointStatus, classify_binding};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeBindingKind {
    Unique,
    Ambiguous,
    None,
}

pub(crate) fn classify_runtime_binding(
    runtime_binding: RuntimeBindingKind,
    pnp_resolved: bool,
    topology_references: &[String],
    package_available: bool,
) -> SystemAudioEndpointStatus {
    match runtime_binding {
        RuntimeBindingKind::Ambiguous => SystemAudioEndpointStatus::Ambiguous,
        RuntimeBindingKind::Unique if !pnp_resolved => SystemAudioEndpointStatus::Unsupported,
        RuntimeBindingKind::Unique if package_available => SystemAudioEndpointStatus::Installable,
        RuntimeBindingKind::Unique => SystemAudioEndpointStatus::ComponentRequired,
        RuntimeBindingKind::None => classify_binding(
            pnp_resolved,
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
    fn unique_runtime_binding_without_reference_requires_component_without_package() {
        assert_eq!(
            classify_runtime_binding(RuntimeBindingKind::Unique, true, &[], false),
            SystemAudioEndpointStatus::ComponentRequired
        );
    }

    #[test]
    fn unique_runtime_binding_without_reference_is_installable_with_package() {
        assert_eq!(
            classify_runtime_binding(RuntimeBindingKind::Unique, true, &[], true),
            SystemAudioEndpointStatus::Installable
        );
    }

    #[test]
    fn unique_runtime_binding_requires_driver_metadata() {
        assert_eq!(
            classify_runtime_binding(RuntimeBindingKind::Unique, false, &[], true),
            SystemAudioEndpointStatus::Unsupported
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
