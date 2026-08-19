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
}
