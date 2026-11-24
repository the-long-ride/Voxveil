use super::{ClockPosition, DEVICE_BUFFER_MS, FRAME_BYTES, RelayBuffer, tests::submit};

pub(super) fn observe_frames(buffer: &mut RelayBuffer, capture: u64, render: u64) {
    let timestamp = capture * 10_000_000 / 48_000;
    buffer.observe_clocks(capture, timestamp, || {
        Some(ClockPosition::new(render, timestamp, 48_000))
    });
}

#[test]
fn matched_clocks_do_not_splice_after_a_phase_shift_across_windows() {
    let mut buffer = RelayBuffer::new(4_800, 48_000);
    let mut padding = buffer.render_capacity;
    let packet = [0; 480 * FRAME_BYTES];
    let mut corrections = 0;

    for tick in 0..3_000 {
        padding -= 480;
        let packets = match tick {
            0 | 1 => 0,
            500 => 3,
            _ => 1,
        };
        for _ in 0..packets {
            buffer.push_capture(&packet);
        }
        if packets != 0 {
            let device_position = (tick + 1) * 480;
            observe_frames(&mut buffer, device_position, device_position);
        }
        let total = padding + buffer.queued_frames();
        submit(&mut buffer, &mut padding, packets * 480);
        corrections += (padding + buffer.queued_frames()).abs_diff(total);
    }

    assert_eq!(corrections, 0);
}

#[test]
fn sustained_clock_drift_preserves_reserve_with_quantized_render_padding() {
    check_drift_reserve(&[-500, -200, 200, 500], 48_000, 2, 3);
}

#[test]
fn sub_deadband_clock_drift_preserves_reserve_for_an_hour() {
    check_drift_reserve(&[-50, 50], 360_000, 4, 5);
}

fn check_drift_reserve(
    ppms: &[isize],
    ticks: u64,
    max_loss_periods: usize,
    max_gain_periods: usize,
) {
    let rate = 48_000;
    let period = 480;
    let packet = [0; 480 * FRAME_BYTES];
    let mut failures = Vec::new();

    for quantum in [1, period] {
        for &ppm in ppms {
            let mut buffer = RelayBuffer::new(rate * DEVICE_BUFFER_MS / 1_000, rate as u32);
            let mut padding = buffer.render_capacity;
            let mut rendered = 0;
            let mut correction = 0;
            let mut min_reserve = buffer.reserve;
            let mut max_reserve = buffer.reserve;
            let mut failure = None;

            for tick in 1..=ticks {
                let capture = tick * period as u64;
                let render = capture * (1_000_000 + ppm) as u64 / 1_000_000;
                let due = render as usize / quantum * quantum;
                let consumed = due - rendered;
                rendered = due;
                if consumed >= padding || buffer.queued_frames() + period > buffer.queue_limit {
                    failure = Some(tick);
                    break;
                }
                padding -= consumed;
                buffer.push_capture(&packet);
                observe_frames(&mut buffer, capture, render);
                let before = padding + buffer.queued_frames();
                submit(&mut buffer, &mut padding, period);
                let queued = buffer.queued_frames();
                correction += (padding + queued) as isize - before as isize;
                min_reserve = min_reserve.min(queued);
                max_reserve = max_reserve.max(queued);
            }

            if failure.is_some()
                || correction.signum() != ppm.signum()
                || min_reserve < buffer.reserve - max_loss_periods * period
                // The slower clock can start one period behind before any drift accumulates.
                || max_reserve > buffer.reserve + max_gain_periods * period
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
