use super::*;

#[test]
fn relay_dsp_requires_stereo_f32() {
    assert!(relay_format_supported(2, 32, true));
    assert!(!relay_format_supported(1, 32, true));
    assert!(!relay_format_supported(2, 16, false));
    assert!(!relay_format_supported(2, 64, true));
}

#[test]
fn relay_capture_packet_has_a_fixed_memory_bound() {
    assert!(capture_packet_within_bound(MAX_CAPTURE_FRAMES));
    assert!(!capture_packet_within_bound(MAX_CAPTURE_FRAMES + 1));
}
