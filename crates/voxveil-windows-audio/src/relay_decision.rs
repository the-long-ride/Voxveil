use voxveil_types::WindowsInterceptionPolicy;
use crate::device::{BackendProbe, EndpointDescriptor, RelayReadiness, WindowsAudioRoute, WindowsInterceptionKind};
use crate::relay_engine::RelayRuntimeState;
use crate::virtual_endpoint::VirtualEndpointKind;

fn interception_kind(kind: VirtualEndpointKind) -> WindowsInterceptionKind {
    match kind {
        VirtualEndpointKind::VoxveilCable => WindowsInterceptionKind::VoxveilCableRelay,
        VirtualEndpointKind::VbCable => WindowsInterceptionKind::VbCableRelay,
        VirtualEndpointKind::EndpointRelay => WindowsInterceptionKind::EndpointRelay,
    }
}

fn route_label(kind: VirtualEndpointKind) -> &'static str {
    match kind {
        VirtualEndpointKind::VoxveilCable => "Voxveil virtual audio relay",
        VirtualEndpointKind::VbCable => "VB-CABLE relay",
        VirtualEndpointKind::EndpointRelay => "Driverless endpoint relay",
    }
}

fn default_route_instruction(kind: VirtualEndpointKind) -> &'static str {
    match kind {
        VirtualEndpointKind::EndpointRelay => crate::route::ENDPOINT_RELAY_INSTRUCTION,
        VirtualEndpointKind::VoxveilCable => {
            "Set Voxveil Input as the Windows default output, then return to Voxveil"
        }
        VirtualEndpointKind::VbCable => {
            "Set CABLE Input as the Windows default output, then return to Voxveil"
        }
    }
}

pub(crate) fn should_sync_apo_after_relay(
    apo_loaded: bool,
    relay_was_active: bool,
    enabled: bool,
) -> bool {
    apo_loaded && relay_was_active && enabled
}

pub(crate) fn should_fail_closed_after_probe(readiness: RelayReadiness) -> bool {
    readiness != RelayReadiness::Ready
}

pub(crate) fn decide_backend(
    apo_loaded: bool,
    selected: Option<(VirtualEndpointKind, &EndpointDescriptor)>,
    physical: Option<&EndpointDescriptor>,
    relay_state: Option<&RelayRuntimeState>,
) -> BackendProbe {
    let physical_route = |interception, source: Option<&EndpointDescriptor>| WindowsAudioRoute {
        interception,
        source_endpoint_id: source.map(|endpoint| endpoint.id.clone()),
        source_display_name: source.map(|endpoint| endpoint.name.clone()),
        physical_output_endpoint_id: physical.map(|endpoint| endpoint.id.clone()),
        physical_output_display_name: physical.map(|endpoint| endpoint.name.clone()),
    };

    if apo_loaded {
        return BackendProbe {
            readiness: RelayReadiness::Ready,
            route: physical_route(Some(WindowsInterceptionKind::Apo), None),
            detail: None,
        };
    }

    let Some((virtual_kind, source)) = selected else {
        return BackendProbe {
            readiness: RelayReadiness::ComponentRequired,
            route: physical_route(None, None),
            detail: Some(
                "Connect a spare Windows output for driverless endpoint relay, or install the standard VB-CABLE virtual audio device or a verified Voxveil virtual audio driver"
                    .into(),
            ),
        };
    };
    let route = physical_route(Some(interception_kind(virtual_kind)), Some(source));

    if !source.is_default {
        return BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            route,
            detail: Some(default_route_instruction(virtual_kind).into()),
        };
    }
    if physical.is_none() || physical.is_some_and(|output| output.id == source.id) {
        return BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            route,
            detail: Some(if virtual_kind == VirtualEndpointKind::EndpointRelay {
                crate::route::ENDPOINT_RELAY_INSTRUCTION.into()
            } else {
                "Select a physical playback endpoint for Voxveil output".into()
            }),
        };
    }

    match relay_state {
        Some(RelayRuntimeState::Running) => BackendProbe {
            readiness: RelayReadiness::Ready,
            route,
            detail: None,
        },
        Some(RelayRuntimeState::Faulted(error)) => BackendProbe {
            readiness: RelayReadiness::Faulted,
            route,
            detail: Some(error.clone()),
        },
        Some(RelayRuntimeState::Starting) => BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            route,
            detail: Some(format!("{} is starting", route_label(virtual_kind))),
        },
        Some(RelayRuntimeState::Stopped) | None => BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            route,
            detail: Some(format!(
                "{} is configured and ready to start",
                route_label(virtual_kind)
            )),
        },
    }
}

pub(crate) fn decide_backend_with_policy(
    policy: WindowsInterceptionPolicy,
    apo_loaded: bool,
    selected: Option<(VirtualEndpointKind, &EndpointDescriptor)>,
    physical: Option<&EndpointDescriptor>,
    relay_state: Option<&RelayRuntimeState>,
) -> BackendProbe {
    match policy {
        WindowsInterceptionPolicy::Automatic => {
            decide_backend(apo_loaded, selected, physical, relay_state)
        }
        WindowsInterceptionPolicy::PhysicalApoOnly if apo_loaded => {
            decide_backend(true, None, physical, None)
        }
        WindowsInterceptionPolicy::PhysicalApoOnly => BackendProbe {
            readiness: if physical.is_some() {
                RelayReadiness::ComponentRequired
            } else {
                RelayReadiness::RoutingRequired
            },
            route: WindowsAudioRoute {
                physical_output_endpoint_id: physical.map(|endpoint| endpoint.id.clone()),
                physical_output_display_name: physical.map(|endpoint| endpoint.name.clone()),
                ..WindowsAudioRoute::default()
            },
            detail: Some(if physical.is_some() {
                "No compatible Voxveil physical APO is loaded on the active physical output endpoint; switch to the automatic route in Settings to use the driver-free relay".into()
            } else {
                "Connect or select a physical playback endpoint before enabling physical APO processing".into()
            }),
        },
        WindowsInterceptionPolicy::Disabled => BackendProbe {
            readiness: RelayReadiness::Unsupported,
            route: WindowsAudioRoute {
                physical_output_endpoint_id: physical.map(|endpoint| endpoint.id.clone()),
                physical_output_display_name: physical.map(|endpoint| endpoint.name.clone()),
                ..WindowsAudioRoute::default()
            },
            detail: Some(
                "System-wide audio interception is disabled while local file playback is selected"
                    .into(),
            ),
        },
    }
}
