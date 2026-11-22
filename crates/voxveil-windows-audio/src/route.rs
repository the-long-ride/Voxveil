use crate::device::EndpointDescriptor;
use crate::virtual_endpoint::classify_virtual_endpoint;

pub(crate) fn select_physical_output<'a>(
    endpoints: &'a [EndpointDescriptor],
    virtual_endpoint_id: &str,
    preferred_id: Option<&str>,
) -> Option<&'a EndpointDescriptor> {
    let is_safe = |endpoint: &EndpointDescriptor| {
        endpoint.id != virtual_endpoint_id && classify_virtual_endpoint(endpoint).is_none()
    };

    preferred_id
        .and_then(|id| {
            endpoints
                .iter()
                .find(|endpoint| endpoint.id == id && is_safe(endpoint))
        })
        .or_else(|| {
            endpoints
                .iter()
                .find(|endpoint| endpoint.is_default && is_safe(endpoint))
        })
        .or_else(|| endpoints.iter().find(|endpoint| is_safe(endpoint)))
}

pub(crate) fn validate_distinct_route(
    source_id: &str,
    physical_id: &str,
) -> Result<(), &'static str> {
    if source_id == physical_id {
        Err("interception endpoint cannot also be the physical output")
    } else {
        Ok(())
    }
}

pub(crate) const ENDPOINT_RELAY_INSTRUCTION: &str =
    "Set a spare output device as the Windows default output and choose a different device for Voxveil output";

pub(crate) fn select_relay_source(
    endpoints: &[EndpointDescriptor],
) -> Option<(crate::virtual_endpoint::VirtualEndpointKind, &EndpointDescriptor)> {
    use crate::virtual_endpoint::{VirtualEndpointKind, find_preferred_virtual_endpoint};
    find_preferred_virtual_endpoint(endpoints).or_else(|| {
        endpoints.iter().find(|endpoint| endpoint.is_default
            && classify_virtual_endpoint(endpoint).is_none())
            .map(|endpoint| (VirtualEndpointKind::EndpointRelay, endpoint))
    })
}

pub(crate) fn select_relay_output<'a>(
    endpoints: &'a [EndpointDescriptor],
    selected: Option<(crate::virtual_endpoint::VirtualEndpointKind, &EndpointDescriptor)>,
    preferred_id: Option<&str>,
) -> Option<&'a EndpointDescriptor> {
    use crate::virtual_endpoint::VirtualEndpointKind;
    let source_id = selected.map(|(_, source)| source.id.as_str()).unwrap_or("");
    if matches!(selected, Some((VirtualEndpointKind::EndpointRelay, _))) {
        return preferred_id.and_then(|id| endpoints.iter().find(|endpoint|
            endpoint.id == id && endpoint.id != source_id
                && classify_virtual_endpoint(endpoint).is_none()));
    }
    select_physical_output(endpoints, source_id, preferred_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn physical(id: &str, name: &str, is_default: bool) -> EndpointDescriptor {
        EndpointDescriptor {
            id: id.into(),
            name: name.into(),
            interface_name: Some("Physical Audio".into()),
            description: Some("Speakers".into()),
            is_default,
        }
    }

    fn vb_cable(id: &str, is_default: bool) -> EndpointDescriptor {
        EndpointDescriptor {
            id: id.into(),
            name: "CABLE Input (VB-Audio Virtual Cable)".into(),
            interface_name: Some("VB-Audio Virtual Cable".into()),
            description: Some("CABLE Input".into()),
            is_default,
        }
    }

    #[test]
    fn preferred_physical_endpoint_wins_when_safe() {
        let endpoints = vec![
            physical("speakers", "Speakers", false),
            physical("headphones", "Headphones", true),
        ];
        let selected = select_physical_output(&endpoints, "cable", Some("speakers")).unwrap();
        assert_eq!(selected.id, "speakers");
    }

    #[test]
    fn non_virtual_default_is_fallback() {
        let endpoints = vec![physical("speakers", "Speakers", true)];
        let selected = select_physical_output(&endpoints, "cable", Some("gone")).unwrap();
        assert_eq!(selected.id, "speakers");
    }

    #[test]
    fn selected_virtual_endpoint_is_never_physical_output() {
        let endpoints = vec![vb_cable("cable", true)];
        assert!(select_physical_output(&endpoints, "cable", Some("cable")).is_none());
    }

    #[test]
    fn another_known_virtual_endpoint_is_not_a_fallback() {
        let endpoints = vec![vb_cable("other-cable", false)];
        assert!(select_physical_output(&endpoints, "source", None).is_none());
    }

    #[test]
    fn identity_collision_is_rejected() {
        assert!(validate_distinct_route("same", "same").is_err());
        assert_eq!(validate_distinct_route("cable", "speakers"), Ok(()));
    }
}
