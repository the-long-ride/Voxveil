use crate::{BackendProbe, RelayReadiness};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsBackendKind {
    Apo,
    Relay,
}

pub fn select_backend(apo: &BackendProbe, relay: &BackendProbe) -> WindowsBackendKind {
    if apo.readiness == RelayReadiness::Ready {
        WindowsBackendKind::Apo
    } else {
        let _ = relay;
        WindowsBackendKind::Relay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(readiness: RelayReadiness) -> BackendProbe {
        BackendProbe {
            readiness,
            physical_output: None,
            detail: None,
        }
    }

    #[test]
    fn ready_apo_has_priority_over_ready_relay() {
        assert_eq!(
            select_backend(&probe(RelayReadiness::Ready), &probe(RelayReadiness::Ready)),
            WindowsBackendKind::Apo
        );
    }

    #[test]
    fn installed_faulted_apo_stays_selected_for_diagnostics() {
        assert_eq!(
            select_backend(&probe(RelayReadiness::Faulted), &probe(RelayReadiness::Ready)),
            WindowsBackendKind::Apo
        );
    }

    #[test]
    fn unavailable_apo_falls_back_to_relay() {
        for readiness in [
            RelayReadiness::ComponentRequired,
            RelayReadiness::RoutingRequired,
            RelayReadiness::Unsupported,
        ] {
            assert_eq!(
                select_backend(&probe(readiness), &probe(RelayReadiness::Ready)),
                WindowsBackendKind::Relay
            );
        }
    }
}
