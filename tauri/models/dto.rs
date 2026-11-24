use serde::Serialize;

use super::catalog::ModelDescriptor;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelStatusDto {
    pub id: String,
    pub display_name: String,
    pub approximate_size_mb: u32,
    pub license: String,
    pub attribution: String,
    pub source: String,
    pub source_revision: String,
    pub installed: bool,
    pub runtime_available: bool,
    pub bundled: bool,
    pub download_available: bool,
    pub consent_required: bool,
}

impl AiModelStatusDto {
    pub fn from_descriptor(model: &ModelDescriptor, installed: bool, bundled: bool) -> Self {
        Self {
            id: model.id.clone(),
            display_name: model.display_name.clone(),
            approximate_size_mb: model.approximate_size_mb,
            license: model.license.clone(),
            attribution: model.attribution.clone(),
            source: model.source.clone(),
            source_revision: model.source_revision.clone(),
            installed: installed || bundled,
            runtime_available: super::runtime_available(),
            bundled,
            download_available: !bundled && model.commercial_use && model.direct_download_only,
            consent_required: !bundled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_status_is_installed_without_consent_or_download_and_fallback_stays_available() {
        let model = crate::models::catalog::find(crate::models::DEFAULT_MODEL_ID).unwrap();
        for (installed, bundled) in [(false, false), (true, false), (true, true)] {
            let status = AiModelStatusDto::from_descriptor(&model, installed, bundled);
            let json = serde_json::to_value(status).unwrap();
            assert_eq!(json["installed"], installed);
            assert_eq!(json["bundled"], bundled);
            assert_eq!(json["consentRequired"], !bundled);
            assert_eq!(json["downloadAvailable"], !bundled);
            assert_eq!(json["id"], "mdx-inst-hq3");
        }
    }
}
