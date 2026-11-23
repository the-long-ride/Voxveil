use crate::device::{EndpointDescriptor, RelayReadiness, WindowsInterceptionKind};
use crate::relay_decision::decide_backend;
use crate::relay_engine::RelayRuntimeState;
use crate::route::{select_relay_output, select_relay_source, ENDPOINT_RELAY_INSTRUCTION};
use crate::virtual_endpoint::VirtualEndpointKind;

fn endpoint(id: &str, is_default: bool) -> EndpointDescriptor {
    EndpointDescriptor {
        id: id.into(), name: id.into(), is_default,
        interface_name: None, description: None,
    }
}

#[test]
fn endpoint_relay_requires_explicit_distinct_output() {
    let endpoints = [endpoint("hdmi", true), endpoint("headphones", false)];
    let selected = select_relay_source(&endpoints);
    assert_eq!(selected.unwrap().0, VirtualEndpointKind::EndpointRelay);
    for preference in [None, Some("hdmi"), Some("missing")] {
        let output = select_relay_output(&endpoints, selected, preference);
        assert!(output.is_none());
        let probe = decide_backend(false, selected, output, None);
        assert_eq!(probe.readiness, RelayReadiness::RoutingRequired);
        assert_eq!(probe.detail.as_deref(), Some(ENDPOINT_RELAY_INSTRUCTION));
    }
    let output = select_relay_output(&endpoints, selected, Some("headphones"));
    let probe = decide_backend(false, selected, output, Some(&RelayRuntimeState::Running));
    assert_eq!(probe.readiness, RelayReadiness::Ready);
    assert_eq!(probe.route.interception, Some(WindowsInterceptionKind::EndpointRelay));
    assert_eq!(probe.route.source_endpoint_id.as_deref(), Some("hdmi"));
    assert_eq!(decide_backend(false, selected, output, None).readiness, RelayReadiness::RoutingRequired);
    let fault = RelayRuntimeState::Faulted("unsupported source format".into());
    let probe = decide_backend(false, selected, output, Some(&fault));
    assert_eq!(probe.readiness, RelayReadiness::Faulted);
    assert_eq!(probe.detail.as_deref(), Some("unsupported source format"));
    assert_eq!(decide_backend(true, selected, output, None).route.interception, Some(WindowsInterceptionKind::Apo));
}

#[test]
fn missing_default_does_not_select_an_arbitrary_source() {
    assert!(select_relay_source(&[]).is_none());
    assert!(select_relay_source(&[endpoint("headphones", false)]).is_none());
    let probe = decide_backend(false, None, None, None);
    assert_eq!(probe.readiness, RelayReadiness::ComponentRequired);
    assert!(probe.detail.unwrap().contains("driverless"));
}

#[test]
fn voxveil_virtual_source_precedes_default_physical_endpoint() {
    let mut voxveil = endpoint("voxveil", false);
    voxveil.name = "Voxveil Input".into();
    voxveil.interface_name = Some("Voxveil Virtual Audio".into());
    let mut endpoints = vec![endpoint("hdmi", true)];
    assert_eq!(select_relay_source(&endpoints).unwrap().0, VirtualEndpointKind::EndpointRelay);
    endpoints.push(voxveil);
    assert_eq!(select_relay_source(&endpoints).unwrap().0, VirtualEndpointKind::VoxveilCable);
}
