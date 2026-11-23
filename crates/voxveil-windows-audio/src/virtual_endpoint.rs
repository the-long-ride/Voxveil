use crate::device::EndpointDescriptor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualEndpointKind {
    VoxveilCable,
    EndpointRelay,
}

pub(crate) fn classify_virtual_endpoint(
    endpoint: &EndpointDescriptor,
) -> Option<VirtualEndpointKind> {
    let name = endpoint.name.trim();
    let interface_name = endpoint.interface_name.as_deref().unwrap_or("").trim();

    let voxveil_name = name.eq_ignore_ascii_case("Voxveil Input");
    let voxveil_device = interface_name.eq_ignore_ascii_case("Voxveil Virtual Audio");
    if voxveil_name && voxveil_device {
        Some(VirtualEndpointKind::VoxveilCable)
    } else {
        None
    }
}

pub(crate) fn find_voxveil_cable(
    endpoints: &[EndpointDescriptor],
) -> Option<&EndpointDescriptor> {
    endpoints.iter().find(|endpoint| {
        classify_virtual_endpoint(endpoint) == Some(VirtualEndpointKind::VoxveilCable)
    })
}

pub(crate) fn find_preferred_virtual_endpoint(
    endpoints: &[EndpointDescriptor],
) -> Option<(VirtualEndpointKind, &EndpointDescriptor)> {
    find_voxveil_cable(endpoints).map(|endpoint| (VirtualEndpointKind::VoxveilCable, endpoint))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(id: &str, name: &str, interface_name: &str, description: &str) -> EndpointDescriptor {
        EndpointDescriptor {
            id: id.into(),
            name: name.into(),
            interface_name: Some(interface_name.into()),
            description: Some(description.into()),
            is_default: false,
        }
    }

    #[test]
    fn recognizes_voxveil_virtual_audio_endpoint() {
        let endpoint = endpoint(
            "voxveil-id",
            "Voxveil Input",
            "Voxveil Virtual Audio",
            "Voxveil Input",
        );
        assert_eq!(
            classify_virtual_endpoint(&endpoint),
            Some(VirtualEndpointKind::VoxveilCable)
        );
    }

    #[test]
    fn selects_only_the_voxveil_virtual_endpoint() {
        let mut endpoints = vec![endpoint("speakers", "Speakers", "USB Audio", "Speakers")];
        assert!(find_preferred_virtual_endpoint(&[]).is_none());
        assert!(find_preferred_virtual_endpoint(&endpoints).is_none());
        endpoints.push(endpoint(
            "voxveil",
            "Voxveil Input",
            "Voxveil Virtual Audio",
            "Voxveil Input",
        ));
        let (kind, selected) = find_preferred_virtual_endpoint(&endpoints).unwrap();
        assert_eq!(kind, VirtualEndpointKind::VoxveilCable);
        assert_eq!(selected.id, "voxveil");
    }

    #[test]
    fn rejects_voxveil_name_without_device_metadata() {
        let endpoint = endpoint("spoof", "Voxveil Input", "USB Audio", "Voxveil Input");
        assert_eq!(classify_virtual_endpoint(&endpoint), None);
    }
}
