#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndpointDescriptor {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayReadiness {
    Ready,
    ComponentRequired,
    RoutingRequired,
    Faulted,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendProbe {
    pub readiness: RelayReadiness,
    pub physical_output: Option<String>,
    pub detail: Option<String>,
}

impl BackendProbe {
    pub fn unsupported() -> Self {
        Self {
            readiness: RelayReadiness::Unsupported,
            physical_output: None,
            detail: None,
        }
    }
}

pub fn is_virtual_render_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("voxveil output")
        || lower.contains("sysvad")
        || lower.contains("virtual audio device (wdm) - tablet")
}

pub fn find_virtual_render(render: &[EndpointDescriptor]) -> Option<&EndpointDescriptor> {
    render
        .iter()
        .find(|endpoint| endpoint.is_default && is_virtual_render_name(&endpoint.name))
        .or_else(|| {
            render
                .iter()
                .find(|endpoint| is_virtual_render_name(&endpoint.name))
        })
}

pub fn choose_endpoints(render: &[EndpointDescriptor]) -> BackendProbe {
    let Some(virtual_render) = find_virtual_render(render) else {
        return component_required(render, "Voxveil Output is not installed");
    };
    let Some(physical) = render
        .iter()
        .find(|endpoint| !is_virtual_render_name(&endpoint.name))
    else {
        return BackendProbe {
            readiness: RelayReadiness::Faulted,
            physical_output: None,
            detail: Some("No physical render endpoint is available".into()),
        };
    };
    if !virtual_render.is_default {
        return BackendProbe {
            readiness: RelayReadiness::RoutingRequired,
            physical_output: Some(physical.name.clone()),
            detail: Some("Set Voxveil Output as the Windows default output".into()),
        };
    }
    BackendProbe {
        readiness: RelayReadiness::Ready,
        physical_output: Some(physical.name.clone()),
        detail: None,
    }
}

fn component_required(render: &[EndpointDescriptor], detail: &str) -> BackendProbe {
    BackendProbe {
        readiness: RelayReadiness::ComponentRequired,
        physical_output: render
            .iter()
            .find(|endpoint| endpoint.is_default && !is_virtual_render_name(&endpoint.name))
            .map(|endpoint| endpoint.name.clone()),
        detail: Some(detail.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(name: &str, is_default: bool) -> EndpointDescriptor {
        EndpointDescriptor {
            id: name.into(),
            name: name.into(),
            is_default,
        }
    }

    #[test]
    fn requires_virtual_render_component() {
        let render = [endpoint("Speakers", true)];
        assert_eq!(
            choose_endpoints(&render).readiness,
            RelayReadiness::ComponentRequired
        );
    }

    #[test]
    fn requires_routing_when_voxveil_output_is_not_default() {
        let render = [
            endpoint("Speakers", true),
            endpoint("Voxveil Output", false),
        ];
        assert_eq!(
            choose_endpoints(&render).readiness,
            RelayReadiness::RoutingRequired
        );
    }

    #[test]
    fn becomes_ready_with_virtual_render_as_default() {
        let render = [
            endpoint("Voxveil Output", true),
            endpoint("Speakers", false),
        ];
        let probe = choose_endpoints(&render);
        assert_eq!(probe.readiness, RelayReadiness::Ready);
        assert_eq!(probe.physical_output.as_deref(), Some("Speakers"));
    }

    #[test]
    fn accepts_sysvad_render_endpoint_in_release_builds() {
        let render = [
            endpoint("SYSVAD (with APO Extensions)", true),
            endpoint("Speakers", false),
        ];
        assert_eq!(choose_endpoints(&render).readiness, RelayReadiness::Ready);
    }

    #[test]
    fn chooses_default_virtual_endpoint_when_multiple_sysvad_outputs_exist() {
        let render = [
            endpoint("Virtual Audio Device (WDM) - Tablet Sample", false),
            endpoint("SYSVAD (with APO Extensions)", true),
            endpoint("Speakers", false),
        ];
        assert_eq!(
            find_virtual_render(&render).map(|endpoint| endpoint.name.as_str()),
            Some("SYSVAD (with APO Extensions)")
        );
    }
}
