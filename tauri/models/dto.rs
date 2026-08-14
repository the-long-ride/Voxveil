use serde::Serialize;

use super::catalog::ModelDescriptor;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelStatusDto {
    pub id: String,
    pub display_name: String,
    pub approximate_size_mb: u32,
    pub license: String,
    pub source: String,
    pub source_revision: String,
    pub installed: bool,
    pub runtime_available: bool,
    pub bundled: bool,
    pub download_available: bool,
    pub consent_required: bool,
}

impl AiModelStatusDto {
    pub fn from_descriptor(model: &ModelDescriptor, installed: bool) -> Self {
        Self {
            id: model.id.clone(),
            display_name: model.display_name.clone(),
            approximate_size_mb: model.approximate_size_mb,
            license: model.license.clone(),
            source: model.source.clone(),
            source_revision: model.source_revision.clone(),
            installed,
            runtime_available: super::AI_RUNTIME_AVAILABLE,
            bundled: false,
            download_available: model.commercial_use && model.direct_download_only,
            consent_required: true,
        }
    }
}
