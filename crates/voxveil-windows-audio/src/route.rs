use crate::device::EndpointDescriptor;
use crate::virtual_endpoint::is_virtual_endpoint;

pub(crate) fn select_physical_output<'a>(
    endpoints: &'a [EndpointDescriptor],
    virtual_endpoint_id: &str,
    preferred_id: Option<&str>,
) -> Option<&'a EndpointDescriptor> {
    let is_safe = |endpoint: &EndpointDescriptor| {
        endpoint.id != virtual_endpoint_id && !is_virtual_endpoint(endpoint)
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
            && !is_virtual_endpoint(endpoint))
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
                && !is_virtual_endpoint(endpoint)));
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

    fn voxveil_cable(id: &str, is_default: bool) -> EndpointDescriptor {
        EndpointDescriptor {
            id: id.into(),
            name: "Voxveil Input".into(),
            interface_name: Some("Voxveil Virtual Audio".into()),
            description: Some("Voxveil Input".into()),
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
        let endpoints = vec![voxveil_cable("cable", true)];
        assert!(select_physical_output(&endpoints, "cable", Some("cable")).is_none());
    }

    #[test]
    fn another_known_virtual_endpoint_is_not_a_fallback() {
        let endpoints = vec![voxveil_cable("other-cable", false)];
        assert!(select_physical_output(&endpoints, "source", None).is_none());
    }

    #[test]
    fn vendor_virtual_default_is_not_a_physical_destination_or_relay_source() {
        let mut vendor = physical("vendor", "CABLE Input (VB-Audio Virtual Cable)", true);
        vendor.interface_name = Some("VB-Audio Virtual Cable".into());
        let mut endpoints = vec![vendor];
        assert!(select_physical_output(&endpoints, "source", None).is_none());
        assert!(select_physical_output(&endpoints, "source", Some("vendor")).is_none());
        assert!(select_relay_source(&endpoints).is_none());
        assert!(!endpoints.iter().any(|item| item.is_default && !is_virtual_endpoint(item)));

        endpoints.push(physical("source", "HDMI", false));
        endpoints.push(physical("speakers", "Speakers", false));
        assert_eq!(
            select_physical_output(&endpoints, "source", Some("vendor")).unwrap().id,
            "speakers"
        );
        let selected = Some((crate::virtual_endpoint::VirtualEndpointKind::EndpointRelay, &endpoints[1]));
        assert!(select_relay_output(&endpoints, selected, Some("vendor")).is_none());
        assert_eq!(
            select_relay_output(&endpoints, selected, Some("speakers")).unwrap().id,
            "speakers"
        );
    }

    #[test]
    fn test_rig_virtual_audio_cable_remains_selectable_as_physical() {
        let mut rig = physical("rig", "Line 1 (Virtual Audio Cable)", true);
        rig.interface_name = Some("Virtual Audio Cable".into());
        rig.description = Some("Eugene Muzychenko".into());
        let endpoints = vec![rig, physical("speakers", "Speakers", false)];
        assert_eq!(select_physical_output(&endpoints, "source", None).unwrap().id, "rig");
        let selected = select_relay_source(&endpoints).unwrap();
        assert_eq!(selected.0, crate::virtual_endpoint::VirtualEndpointKind::EndpointRelay);
        assert_eq!(selected.1.id, "rig");
        assert_eq!(
            select_relay_output(&endpoints, Some(selected), Some("speakers")).unwrap().id,
            "speakers"
        );
        let source = Some((crate::virtual_endpoint::VirtualEndpointKind::EndpointRelay, &endpoints[1]));
        assert_eq!(select_relay_output(&endpoints, source, Some("rig")).unwrap().id, "rig");
    }

    #[test]
    fn identity_collision_is_rejected() {
        assert!(validate_distinct_route("same", "same").is_err());
        assert_eq!(validate_distinct_route("cable", "speakers"), Ok(()));
    }
}
