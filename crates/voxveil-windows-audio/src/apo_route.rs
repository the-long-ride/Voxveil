use crate::device::EndpointDescriptor;

pub(crate) fn apo_covers_default_endpoint(
    loaded_instances: u32,
    installed_endpoint_id: Option<&str>,
    endpoints: &[EndpointDescriptor],
) -> bool {
    if loaded_instances == 0 {
        return false;
    }
    let Some(installed_endpoint_id) = installed_endpoint_id else {
        return false;
    };
    endpoints.iter().any(|endpoint| {
        endpoint.is_default && endpoint.id.eq_ignore_ascii_case(installed_endpoint_id)
    })
}

#[cfg(windows)]
mod install_state {
    use std::path::Path;

    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ApoInstallState {
        endpoint_id: Option<String>,
        binding_mode: String,
    }

    pub(crate) fn load_installed_apo_endpoint(
        system_audio_directory: &Path,
    ) -> Result<Option<String>, String> {
        let path = system_audio_directory.join("install-state.json");
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "failed to read Voxveil APO install state at {}: {error}",
                    path.display()
                ));
            }
        };
        let state: ApoInstallState = serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "Voxveil APO install state at {} is invalid: {error}",
                path.display()
            )
        })?;
        if !matches!(
            state.binding_mode.as_str(),
            "capx-extension" | "legacy-runtime-interface" | "legacy-reference"
        ) {
            return Err(format!(
                "Voxveil APO install state has an unknown bindingMode: {}",
                state.binding_mode
            ));
        }

        let endpoint_id = state
            .endpoint_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if endpoint_id.is_none() {
            if state.binding_mode == "legacy-reference" {
                return Ok(None);
            }
            return Err("Voxveil APO install state has no endpointId for an endpoint-scoped binding".into());
        }
        Ok(endpoint_id.map(str::to_string))
    }
}

#[cfg(windows)]
pub(crate) use install_state::load_installed_apo_endpoint;

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(id: &str, is_default: bool) -> EndpointDescriptor {
        EndpointDescriptor {
            id: id.into(),
            name: id.into(),
            interface_name: None,
            description: None,
            is_default,
        }
    }

    #[test]
    fn loaded_count_is_not_enough_without_install_endpoint_identity() {
        let endpoints = vec![endpoint("speakers", true)];
        assert!(!apo_covers_default_endpoint(1, None, &endpoints));
    }

    #[test]
    fn loaded_apo_covers_only_the_installed_default_endpoint() {
        let endpoints = vec![endpoint("speakers", true), endpoint("headphones", false)];
        assert!(apo_covers_default_endpoint(1, Some("speakers"), &endpoints));
        assert!(!apo_covers_default_endpoint(1, Some("headphones"), &endpoints));
    }

    #[test]
    fn stale_loaded_instance_does_not_cover_a_new_virtual_default() {
        let endpoints = vec![endpoint("speakers", false), endpoint("voxveil", true)];
        assert!(!apo_covers_default_endpoint(1, Some("speakers"), &endpoints));
    }

    #[test]
    fn zero_loaded_instances_never_cover_the_default() {
        let endpoints = vec![endpoint("speakers", true)];
        assert!(!apo_covers_default_endpoint(0, Some("speakers"), &endpoints));
    }
}
