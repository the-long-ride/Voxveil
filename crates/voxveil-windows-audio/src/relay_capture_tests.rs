use super::*;

#[test]
fn each_new_stream_accepts_a_flagged_nonzero_origin_without_resync() {
    for origin in [0, 149_760, 12_000_000] {
        for bad_timestamp in [false, true] {
            let mut timeline = CaptureTimeline::default();
            let first = timeline.observe(origin, 480, true, bad_timestamp, false);
            assert!(first.first && first.reset_clock);
            assert!(!first.resync_ai);
            assert_eq!(first.gap, 0);
            assert_eq!(first.previous_end, None);
            assert!(
                !timeline
                    .observe(origin + 480, 480, false, false, false)
                    .resync_ai
            );
        }
    }
}

#[test]
fn clock_metadata_errors_do_not_mean_audio_was_lost() {
    let mut timeline = CaptureTimeline::default();
    timeline.observe(10_000, 480, false, false, false);
    let invalid = timeline.observe(0, 480, false, true, false);
    assert!(invalid.reset_clock);
    assert!(!invalid.resync_ai);
    assert!(!timeline.observe(10_960, 480, false, false, false).resync_ai);
    assert!(!timeline.observe(11_440, 480, false, false, false).resync_ai);
}

#[test]
fn real_losses_and_position_jumps_resync_but_idle_resume_does_not() {
    for (position, flagged) in [(480, true), (600, false), (10_000, false), (0, false)] {
        let mut timeline = CaptureTimeline::default();
        timeline.observe(0, 480, false, false, false);
        let loss = timeline.observe(position, 480, flagged, false, false);
        assert!(loss.resync_ai && loss.reset_clock);
        assert!(
            !timeline
                .observe(position + 480, 480, false, false, false)
                .resync_ai
        );
    }
    let mut timeline = CaptureTimeline::default();
    timeline.observe(99_000, 480, false, false, false);
    let resume = timeline.observe(2_000_000, 480, true, false, true);
    assert!(resume.reset_clock);
    assert!(!resume.resync_ai);
    assert_eq!(resume.gap, 0);
}
