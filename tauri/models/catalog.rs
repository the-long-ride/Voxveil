use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptor {
    #[serde(default)]
    pub bundled_by_default: bool,
    pub id: String,
    pub display_name: String,
    pub source: String,
    pub source_revision: String,
    pub license: String,
    pub attribution: String,
    pub license_urls: Vec<String>,
    pub files: Vec<ModelFile>,
    pub approximate_size_mb: u32,
    pub commercial_use: bool,
    pub direct_download_only: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFile {
    pub file_name: String,
    pub download_url: String,
    pub sha256: String,
    pub max_bytes: u64,
}

#[derive(Deserialize)]
struct Catalog {
    models: Vec<ModelDescriptor>,
}

pub const DEFAULT_MODEL_ID: &str = "mdx-inst-hq3";

pub fn find(model_id: &str) -> Result<ModelDescriptor, String> {
    let catalog: Catalog = serde_json::from_str(include_str!("catalog.json"))
        .map_err(|_| "AI model catalog is invalid".to_string())?;
    catalog
        .models
        .into_iter()
        .find(|model| model.id == model_id)
        .ok_or_else(|| "AI model is not approved by this Voxveil build".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_is_pinned_and_commercially_approved() {
        assert_eq!(DEFAULT_MODEL_ID, "mdx-inst-hq3");
        for (id, files) in [("spleeter-2stems-fp16", 2), (DEFAULT_MODEL_ID, 1)] {
            let model = find(id).expect("bundled model must exist");
            assert_eq!(model.files.len(), files);
            for file in &model.files {
                assert!(file.download_url.starts_with("https://"));
                assert_eq!(file.sha256.len(), 64);
            }
            assert!(!model.attribution.is_empty());
            assert!(model.commercial_use);
            assert!(model.direct_download_only);
            assert!(model.bundled_by_default);
        }
    }
}
