use super::*;

fn candidate(device_instance_id: &str, interface_path: &str, alias_match: bool) -> TopologyCandidate {
    TopologyCandidate {
        device_instance_id: device_instance_id.into(),
        interface_path: interface_path.into(),
        audio_interface_path: alias_match.then(|| format!("{interface_path}-audio")),
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
fn adapter_interface_path_selects_owning_devnode() {
    let expected = candidate(
        "HDAUDIO\\FUNC_01",
        "\\\\?\\hdaudio#func_01#{dda54a40-1e4c-11d1-a050-405705c10000}",
        true,
    );
    let selection = select_topology_candidate(
        "\\\\?\\HDAUDIO#FUNC_01#{DDA54A40-1E4C-11D1-A050-405705C10000}",
        &[expected.clone()],
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
fn selected_candidate_retains_audio_alias_path() {
    let expected = TopologyCandidate {
        device_instance_id: "HDAUDIO\\FUNC_01".into(),
        interface_path: "topology-interface".into(),
        audio_interface_path: Some("audio-interface".into()),
        alias_match: true,
    };
    let selection = select_topology_candidate("HDAUDIO\\FUNC_01", &[expected.clone()]);
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
