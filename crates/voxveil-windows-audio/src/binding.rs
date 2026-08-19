#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::SystemAudioEndpointStatus;

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
