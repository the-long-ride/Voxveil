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
    assert_eq!(buffer.queued_frames(), buffer.reserve - 576);

    padding -= 480;
    buffer.push_capture(&packet);
    buffer.push_capture(&packet);
    submit(&mut buffer, &mut padding, 960);
    assert_eq!(padding, buffer.render_capacity);
    assert_eq!(buffer.queued_frames(), buffer.reserve - 96);
}

#[test]
fn render_stays_fed_through_logged_worker_stall_and_capture_hole() {
    let rate = 48_000;
    let period = 480;
    let render_capacity = rate * DEVICE_BUFFER_MS / 1_000;
    let mut buffer = RelayBuffer::new(render_capacity, rate as u32);
    let mut padding = render_capacity;
    buffer.bytes.iter_mut().for_each(|byte| *byte = 1);

    // Relay-02: no write for 86.56 ms. Only device padding can play while the worker is stalled.
    let stalled_frames = (86_560 * rate).div_ceil(1_000_000);
    let consumed = stalled_frames.div_ceil(period) * period;
    assert!(padding > consumed, "device ran dry during the stall");
    padding -= consumed;

    // Relay-09: the next source packet exposed a 2,304-frame hole. Stress the reserve by
    // combining the two extremes and giving capture a whole packet period to resume.
    let mut timeline = CaptureTimeline::default();
    assert_eq!(timeline.missing_frames(149_760, 96, true, false), 0);
    let missing = 2_304_usize;
    for _ in 0..missing.div_ceil(period) {
        submit(&mut buffer, &mut padding, 0);
        assert!(padding > period, "render ran dry before the hole was known");
        padding -= period;
    }
    let gap = timeline.missing_frames(152_160, period, true, false);
    assert_eq!(gap, missing);
    let total = padding + buffer.queued_frames();
    buffer.push_capture(&vec![0; gap * FRAME_BYTES]);
    buffer.push_capture(&[2; 480 * FRAME_BYTES]);
    assert!(
        buffer.bytes.iter().copied().eq((0..gap * FRAME_BYTES)
            .map(|_| 0)
            .chain([2; 480 * FRAME_BYTES]))
    );
    submit(&mut buffer, &mut padding, period);
    assert_eq!(padding + buffer.queued_frames(), total + gap + period);
}

#[test]
fn complete_underrun_rebuilds_the_reserve_without_discarding_audio() {
    let mut buffer = RelayBuffer::new(8, 48_000);
    buffer.render_queue().clear();
    let packet = [1; 3 * FRAME_BYTES];
    buffer.push_capture(&packet);

    buffer.balance(0, 3);

    let target = buffer.render_capacity + buffer.reserve;
    let silence_bytes = (target - 3) * FRAME_BYTES;
    assert_eq!(buffer.queued_frames(), target);
    assert!(buffer.bytes.range(..silence_bytes).all(|byte| *byte == 0));
    assert!(buffer.bytes.range(silence_bytes..).copied().eq(packet));
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
        let target = padding + buffer.reserve;
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
            assert!(total.abs_diff(target) <= 720, "tick {tick}: {total}");
        }
        assert_eq!(correction.signum(), drift_frames);
    }
}

#[test]
fn sustained_clock_drift_preserves_reserve_with_quantized_render_padding() {
    let rate = 48_000;
    let period = 480;
    let packet = [0; 480 * FRAME_BYTES];
    let mut failures = Vec::new();

    for quantum in [1, period] {
        for ppm in [-500_isize, -200, 200, 500] {
            let mut buffer = RelayBuffer::new(rate * DEVICE_BUFFER_MS / 1_000, rate as u32);
            let mut padding = buffer.render_capacity;
            let mut rendered = 0;
            let mut correction = 0;
            let mut min_reserve = buffer.reserve;
            let mut max_reserve = buffer.reserve;
            let mut failure = None;

            for tick in 1..=48_000_u64 {
                let due = (tick * period as u64 * (1_000_000 + ppm) as u64 / 1_000_000) as usize
                    / quantum
                    * quantum;
                let consumed = due - rendered;
                rendered = due;
                if consumed >= padding || buffer.queued_frames() + period > buffer.queue_limit {
                    failure = Some(tick);
                    break;
                }
                padding -= consumed;
                buffer.push_capture(&packet);
                let before = padding + buffer.queued_frames();
                submit(&mut buffer, &mut padding, period);
                let queued = buffer.queued_frames();
                correction += (padding + queued) as isize - before as isize;
                min_reserve = min_reserve.min(queued);
                max_reserve = max_reserve.max(queued);
            }

            if failure.is_some()
                || correction.signum() != ppm.signum()
                || min_reserve < buffer.reserve - 2 * period
                // The slower clock can start one period behind before any drift accumulates.
                || max_reserve > buffer.reserve + 3 * period
            {
                failures.push(format!(
                    "{ppm:+} ppm, {quantum}-frame quantum: reserve {min_reserve}..={max_reserve}, \
                     net correction {correction}, first depletion/overflow tick {failure:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
    assert_eq!(buffer.queued_frames(), MIN_QUEUE_LIMIT_FRAMES);
    assert_eq!(buffer.bytes.capacity(), capacity);
    assert!(
        buffer
            .bytes
            .iter()
            .copied()
            .eq(older.into_iter().chain(newer))
    );
}
