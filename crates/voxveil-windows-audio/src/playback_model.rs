pub(crate) fn uses_model(active: Option<voxveil_types::AiModel>, wanted: &str) -> bool {
    active.is_some_and(|model| model.id() == wanted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxveil_types::AiModel;
    #[test]
    fn removal_does_not_match_dsp_or_a_different_active_model() {
        assert!(!uses_model(None, AiModel::Spleeter.id()));
        assert!(!uses_model(Some(AiModel::MdxInstHq3), AiModel::Spleeter.id()));
        assert!(uses_model(Some(AiModel::Spleeter), AiModel::Spleeter.id()));
        assert!(!uses_model(Some(AiModel::Spleeter), "unknown"));
    }
}
