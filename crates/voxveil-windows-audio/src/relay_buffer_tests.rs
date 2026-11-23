use super::*;

fn submit(buffer: &mut RelayBuffer, padding: &mut usize, captured_frames: usize) {
    buffer.balance(*padding, captured_frames);
    let frames_to_write = (buffer.render_capacity - *padding).min(buffer.queued_frames());
    buffer.render_queue().drain(..frames_to_write * FRAME_BYTES);
    *padding += frames_to_write;
}

fn stereo_frame(left: f32, right: f32) -> [u8; FRAME_BYTES] {
    let mut frame = [0; FRAME_BYTES];
    frame[..4].copy_from_slice(&left.to_le_bytes());
    frame[4..].copy_from_slice(&right.to_le_bytes());
    frame
}

#[test]
fn render_reserve_covers_a_late_capture_poll_and_batched_catchup() {
    let mut buffer = RelayBuffer::new(1_536, 48_000);
    let mut padding = buffer.render_capacity;
    let packet = [1; 480 * FRAME_BYTES];

    padding -= 480;
    buffer.push_capture(&packet);
    submit(&mut buffer, &mut padding, 480);
    assert_eq!(padding, buffer.render_capacity);

    // The Windows trace had 480–576 missing render frames after the relay ran out of audio.
    padding -= 576;
    submit(&mut buffer, &mut padding, 0);
    assert_eq!(padding, buffer.render_capacity);
    assert_eq!(buffer.queued_frames(), 192);

    padding -= 480;
    buffer.push_capture(&packet);
    buffer.push_capture(&packet);
    submit(&mut buffer, &mut padding, 960);
    assert_eq!(padding, buffer.render_capacity);
    assert_eq!(buffer.queued_frames(), 672);
}

#[test]
fn complete_underrun_rebuilds_the_reserve_without_discarding_audio() {
    let mut buffer = RelayBuffer::new(8, 48_000);
    buffer.render_queue().clear();
    let packet = [1; 3 * FRAME_BYTES];
    buffer.push_capture(&packet);

    buffer.balance(0, 3);

    assert_eq!(buffer.queued_frames(), 12);
    assert!(buffer.bytes.range(..9 * FRAME_BYTES).all(|byte| *byte == 0));
    assert!(buffer.bytes.range(9 * FRAME_BYTES..).copied().eq(packet));
}

#[test]
fn capture_positions_recover_only_known_missing_frames() {
    let mut timeline = CaptureTimeline::default();
    assert_eq!(timeline.missing_frames(866_304, 96, true, false), 0);
    assert_eq!(timeline.missing_frames(866_400, 96, true, false), 0);
    // Real pilot trace: the discontinuity was flagged on the preceding packet.
    assert_eq!(timeline.missing_frames(866_880, 480, false, false), 384);
    assert_eq!(timeline.missing_frames(867_460, 480, true, false), 100);
    assert_eq!(timeline.missing_frames(10, 480, true, true), 0);
    assert_eq!(timeline.missing_frames(920_000, 480, false, false), 0);
    assert_eq!(timeline.missing_frames(999_000, 480, true, false), 0);
    assert_eq!(timeline.missing_frames(320, 480, true, false), 0);
    assert_eq!(timeline.missing_frames(864, 480, false, false), 64);
}

#[test]
fn clock_drift_keeps_the_fifo_in_a_bounded_band_in_both_directions() {
    let packet = [0; 480 * FRAME_BYTES];
    for drift_frames in [-1, 1] {
        let mut buffer = RelayBuffer::new(1_440, 48_000);
        let mut padding = buffer.render_capacity;
        let mut correction = 0;
        for tick in 1..=8_000 {
            let render_per_tick =
                480_usize.saturating_add_signed(if tick % 10 == 0 { drift_frames } else { 0 });
            assert!(padding >= render_per_tick, "underrun at tick {tick}");
            padding -= render_per_tick;
            buffer.push_capture(&packet);
            let before_balance = padding + buffer.queued_frames();
            submit(&mut buffer, &mut padding, 480);
            let total = padding + buffer.queued_frames();
            correction += total as isize - before_balance as isize;
            assert!((1_440..=2_880).contains(&total), "tick {tick}: {total}");
        }
        assert_eq!(correction.signum(), drift_frames);
    }
}

#[test]
fn faster_render_clock_preserves_reserve_for_late_capture() {
    let mut buffer = RelayBuffer::new(1_536, 48_000);
    let mut padding = buffer.render_capacity;
    let packet = [0; 480 * FRAME_BYTES];

    // One extra render frame per ten capture packets is about 208 ppm at 48 kHz.
    for tick in 1..=8_000 {
        let render_frames = 480 + usize::from(tick % 10 == 0);
        padding -= render_frames;
        buffer.push_capture(&packet);
        submit(&mut buffer, &mut padding, 480);
    }

    let reserve_after_drift = buffer.queued_frames();
    for period in 1..=4 {
        assert!(
            padding >= 480,
            "underrun in period {period} without capture: {padding} frames of padding, \
             {reserve_after_drift} frames of reserve after drift"
        );
        padding -= 480;
        submit(&mut buffer, &mut padding, 0);
    }
    assert!(reserve_after_drift >= buffer.reserve - 480);
}

#[test]
fn rate_matched_vac_bursts_and_packet_phase_shift_never_splice() {
    let mut buffer = RelayBuffer::new(1_440, 48_000);
    let mut padding = buffer.render_capacity;
    let packet = [0; 480 * FRAME_BYTES];

    // Relay-07 had 1,680 frames at capture polls before falling to a 1,200-frame plateau.
    buffer.render_queue().truncate(240 * FRAME_BYTES);
    let mut process = |render_frames: usize, capture_packets: usize| {
        padding -= render_frames;
        for _ in 0..capture_packets {
            buffer.push_capture(&packet);
        }
        let total = padding + buffer.queued_frames();
        submit(&mut buffer, &mut padding, capture_packets * 480);
        assert_eq!(
            padding + buffer.queued_frames(),
            total,
            "spliced at fill {total}"
        );
    };
    for tick in 0..2_000 {
        if tick == 1_334 {
            process(480, 0);
        }
        if tick % 97 == 0 {
            // The relay-06/08 traces also had occasional one-packet positive excursions.
            process(0, 1);
            process(960, 1);
        } else {
            process(480, 1);
        }
    }
    process(0, 1);
    assert_eq!(padding + buffer.queued_frames(), 1_680);
}

#[test]
fn drift_adjustment_interpolates_complete_stereo_frames() {
    let mut buffer = RelayBuffer::new(8, 48_000);
    buffer.render_queue().clear();
    let a = stereo_frame(0.25, -0.75);
    let midpoint = stereo_frame(0.5, -0.5);
    let b = stereo_frame(0.75, -0.25);
    buffer.push_capture(&[a, b].concat());
    buffer.splice_tail(true);
    assert!(buffer.bytes.iter().copied().eq([a, midpoint, b].concat()));

    buffer.push_capture(&[a, b].repeat(3).concat());
    buffer.splice_tail(false);
    let start = buffer.bytes.len() - 2 * FRAME_BYTES;
    assert!(
        buffer
            .bytes
            .range(start..)
            .copied()
            .eq([b, midpoint].concat())
    );
}

#[test]
fn sustained_overrun_drops_old_whole_frames_before_capacity_growth() {
    let mut buffer = RelayBuffer::new(16, 48_000);
    let capacity = buffer.bytes.capacity();
    let older = [1; MAX_CAPTURE_FRAMES * FRAME_BYTES];
    let newer = [2; MAX_CAPTURE_FRAMES * FRAME_BYTES];
    buffer.push_capture(&older);
    buffer.push_capture(&newer);
    assert_eq!(buffer.queued_frames(), MAX_QUEUE_FRAMES);
    assert_eq!(buffer.bytes.capacity(), capacity);
    assert!(
        buffer
            .bytes
            .iter()
            .copied()
            .eq(older.into_iter().chain(newer))
    );
}
