#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TopologyCandidate {
    pub(crate) device_instance_id: String,
    pub(crate) interface_path: String,
    pub(crate) alias_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CandidateSelection {
    Unique(TopologyCandidate),
    Ambiguous,
    None,
}

pub(crate) fn select_topology_candidate(
    adapter_device_id: &str,
    candidates: &[TopologyCandidate],
) -> CandidateSelection {
    let same_device: Vec<_> = candidates
        .iter()
        .filter(|candidate| candidate.device_instance_id.eq_ignore_ascii_case(adapter_device_id))
        .cloned()
        .collect();

    if same_device.is_empty() {
        return CandidateSelection::None;
    }

    let alias_matches: Vec<_> = same_device
        .iter()
        .filter(|candidate| candidate.alias_match)
        .cloned()
        .collect();

    match alias_matches.len() {
        1 => CandidateSelection::Unique(alias_matches[0].clone()),
        n if n > 1 => CandidateSelection::Ambiguous,
        _ => match same_device.len() {
            1 => CandidateSelection::Unique(same_device[0].clone()),
            _ => CandidateSelection::Ambiguous,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(device_instance_id: &str, interface_path: &str, alias_match: bool) -> TopologyCandidate {
        TopologyCandidate {
            device_instance_id: device_instance_id.into(),
            interface_path: interface_path.into(),
            alias_match,
        }
    }

    #[test]
    fn unique_same_device_candidate_wins() {
        let expected = candidate("HDAUDIO\\FUNC_01", "topology-a", false);
        let selection = select_topology_candidate(
            "hdaudio\\func_01",
            &[
                expected.clone(),
                candidate("HDAUDIO\\OTHER", "topology-b", true),
            ],
        );
        assert_eq!(selection, CandidateSelection::Unique(expected));
    }

    #[test]
    fn alias_match_wins_among_same_device_candidates() {
        let expected = candidate("HDAUDIO\\FUNC_01", "topology-alias", true);
        let selection = select_topology_candidate(
            "HDAUDIO\\FUNC_01",
            &[
                candidate("HDAUDIO\\FUNC_01", "topology-plain", false),
                expected.clone(),
            ],
        );
        assert_eq!(selection, CandidateSelection::Unique(expected));
    }

    #[test]
    fn indistinguishable_same_device_candidates_are_ambiguous() {
        let selection = select_topology_candidate(
            "HDAUDIO\\FUNC_01",
            &[
                candidate("HDAUDIO\\FUNC_01", "topology-a", false),
                candidate("hdaudio\\func_01", "topology-b", false),
            ],
        );
        assert_eq!(selection, CandidateSelection::Ambiguous);
    }

    #[test]
    fn unrelated_candidates_do_not_match() {
        let selection = select_topology_candidate(
            "HDAUDIO\\FUNC_01",
            &[candidate("HDAUDIO\\OTHER", "topology-b", true)],
        );
        assert_eq!(selection, CandidateSelection::None);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_topology_interface_enumeration_is_structured() {
        assert!(enumerate_topology_interfaces().is_ok());
    }
}
