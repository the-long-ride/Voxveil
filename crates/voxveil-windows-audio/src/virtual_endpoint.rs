use crate::device::EndpointDescriptor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualEndpointKind {
    VbCable,
    VoxveilCable,
}

pub(crate) fn classify_virtual_endpoint(
    endpoint: &EndpointDescriptor,
) -> Option<VirtualEndpointKind> {
    let name = endpoint.name.trim();
    let description = endpoint.description.as_deref().unwrap_or("").trim();
    let interface_name = endpoint.interface_name.as_deref().unwrap_or("").trim();

    let voxveil_name = name.eq_ignore_ascii_case("Voxveil Input");
    let voxveil_device = interface_name.eq_ignore_ascii_case("Voxveil Virtual Audio");
    if voxveil_name && voxveil_device {
        return Some(VirtualEndpointKind::VoxveilCable);
    }

    let canonical_full_name = name.eq_ignore_ascii_case("CABLE Input (VB-Audio Virtual Cable)");
    let canonical_short_name = name.eq_ignore_ascii_case("CABLE Input");
    let canonical_description = description.eq_ignore_ascii_case("CABLE Input");
    let vendor_metadata = interface_name
        .to_ascii_lowercase()
        .contains("vb-audio virtual cable");

    if canonical_full_name || (canonical_short_name && canonical_description && vendor_metadata) {
        Some(VirtualEndpointKind::VbCable)
    } else {
        None
    }
}

pub(crate) fn find_standard_vb_cable(
    endpoints: &[EndpointDescriptor],
) -> Option<&EndpointDescriptor> {
    endpoints.iter().find(|endpoint| {
        classify_virtual_endpoint(endpoint) == Some(VirtualEndpointKind::VbCable)
    })
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
    if let Some(endpoint) = find_voxveil_cable(endpoints) {
        return Some((VirtualEndpointKind::VoxveilCable, endpoint));
    }
    find_standard_vb_cable(endpoints).map(|endpoint| (VirtualEndpointKind::VbCable, endpoint))
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
    fn recognizes_standard_vb_cable_name() {
        let endpoint = endpoint(
            "cable",
            "CABLE Input (VB-Audio Virtual Cable)",
            "VB-Audio Virtual Cable",
            "CABLE Input",
        );
        assert_eq!(
            classify_virtual_endpoint(&endpoint),
            Some(VirtualEndpointKind::VbCable)
        );
    }

    #[test]
    fn recognizes_short_name_only_with_vendor_metadata() {
        let endpoint = endpoint(
            "cable",
            "CABLE Input",
            "VB-Audio Virtual Cable",
            "CABLE Input",
        );
        assert_eq!(
            classify_virtual_endpoint(&endpoint),
            Some(VirtualEndpointKind::VbCable)
        );
    }

    #[test]
    fn recognizes_multichannel_vb_cable_render_variant() {
        let endpoint = endpoint(
            "cable-16ch",
            "CABLE In 16ch (VB-Audio Virtual Cable)",
            "VB-Audio Virtual Cable",
            "CABLE In 16ch",
        );
        assert_eq!(
            classify_virtual_endpoint(&endpoint),
            Some(VirtualEndpointKind::VbCable)
        );
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
    fn prefers_voxveil_endpoint_over_vb_cable() {
        let endpoints = vec![
            endpoint(
                "vb",
                "CABLE Input (VB-Audio Virtual Cable)",
                "VB-Audio Virtual Cable",
                "CABLE Input",
            ),
            endpoint(
                "voxveil",
                "Voxveil Input",
                "Voxveil Virtual Audio",
                "Voxveil Input",
            ),
        ];
        let (kind, selected) = find_preferred_virtual_endpoint(&endpoints).unwrap();
        assert_eq!(kind, VirtualEndpointKind::VoxveilCable);
        assert_eq!(selected.id, "voxveil");
    }

    #[test]
    fn falls_back_to_vb_cable_when_voxveil_endpoint_is_absent() {
        let endpoints = vec![endpoint(
            "vb",
            "CABLE Input (VB-Audio Virtual Cable)",
            "VB-Audio Virtual Cable",
            "CABLE Input",
        )];
        let (kind, selected) = find_preferred_virtual_endpoint(&endpoints).unwrap();
        assert_eq!(kind, VirtualEndpointKind::VbCable);
        assert_eq!(selected.id, "vb");
    }

    #[test]
    fn rejects_voxveil_name_without_device_metadata() {
        let endpoint = endpoint("spoof", "Voxveil Input", "USB Audio", "Voxveil Input");
        assert_eq!(classify_virtual_endpoint(&endpoint), None);
    }

    #[test]
    fn rejects_generic_cable_named_device() {
        let endpoint = endpoint("other", "My CABLE Input", "USB Audio", "Speakers");
        assert_eq!(classify_virtual_endpoint(&endpoint), None);
    }

    #[test]
    fn rejects_short_name_without_vendor_metadata() {
        let endpoint = endpoint("other", "CABLE Input", "USB Audio", "CABLE Input");
        assert_eq!(classify_virtual_endpoint(&endpoint), None);
    }

    #[test]
    fn ignores_vb_cable_capture_name_on_render_list() {
        let endpoint = endpoint(
            "capture-name",
            "CABLE Output (VB-Audio Virtual Cable)",
            "VB-Audio Virtual Cable",
            "CABLE Output",
        );
        assert_eq!(classify_virtual_endpoint(&endpoint), None);
    }
}
