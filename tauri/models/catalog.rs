use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub file_name: String,
    pub download_url: String,
    pub source: String,
    pub source_revision: String,
    pub license: String,
    pub sha256: String,
    pub approximate_size_mb: u32,
    pub max_bytes: u64,
    pub commercial_use: bool,
    pub direct_download_only: bool,
}

#[derive(Deserialize)]
struct Catalog {
    models: Vec<ModelDescriptor>,
}

pub const DEFAULT_MODEL_ID: &str = "htdemucs-ft-vocals-fp16";

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
        let model = find(DEFAULT_MODEL_ID).expect("default model must exist");
        assert!(model.download_url.starts_with("https://"));
        assert_eq!(model.sha256.len(), 64);
        assert!(model.commercial_use);
        assert!(model.direct_download_only);
    }
}
