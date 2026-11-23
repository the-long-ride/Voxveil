use std::collections::VecDeque;

#[path = "relay_clock.rs"]
mod clock;
#[cfg(windows)]
pub(crate) use clock::RenderClock;
use clock::{ClockPosition, DriftTracker};

pub(crate) const FRAME_BYTES: usize = 8;
pub(crate) const MAX_CAPTURE_FRAMES: usize = 4096;
pub(crate) const DEVICE_BUFFER_MS: usize = 100;
const RESERVE_MS: usize = 64;
const MIN_QUEUE_LIMIT_FRAMES: usize = 8192;

#[derive(Default)]
pub(crate) struct CaptureTimeline {
    next_frame: Option<u64>,
}

impl CaptureTimeline {
    pub(crate) fn missing_frames(
        &mut self,
        position: u64,
        frames: usize,
        discontinuity: bool,
        timestamp_error: bool,
    ) -> usize {
        if timestamp_error {
            self.next_frame = None;
            return 0;
        }

        let previous_end = self.next_frame;
        self.next_frame = position.checked_add(frames as u64);
        let Some(expected) = previous_end else {
            return 0;
        };
        if !discontinuity && position == expected {
            return 0;
        }

        // A flag alone gives no gap length. Rebase on backwards or implausibly large jumps.
        match position.checked_sub(expected) {
            Some(gap) if gap <= MAX_CAPTURE_FRAMES as u64 => gap as usize,
            _ => 0,
        }
    }
}

pub(crate) struct RelayBuffer {
    bytes: VecDeque<u8>,
    render_capacity: usize,
    reserve: usize,
    queue_limit: usize,
    sample_rate: usize,
    drift: DriftTracker,
    reserve_correction: ReserveCorrection,
}

impl RelayBuffer {
    pub(crate) fn new(render_capacity: usize, sample_rate: u32) -> Self {
        let sample_rate = sample_rate as usize;
        let reserve = (sample_rate * RESERVE_MS).div_ceil(1_000);
        let queue_limit =
            (render_capacity + reserve + MAX_CAPTURE_FRAMES).max(MIN_QUEUE_LIMIT_FRAMES);
        let mut bytes = VecDeque::with_capacity(queue_limit * FRAME_BYTES);
        bytes.resize(reserve * FRAME_BYTES, 0);
        Self {
            bytes,
            render_capacity,
            reserve,
            queue_limit,
            sample_rate,
            drift: DriftTracker::default(),
            reserve_correction: ReserveCorrection::default(),
        }
    }

    pub(crate) fn queued_frames(&self) -> usize {
        self.bytes.len() / FRAME_BYTES
    }

    pub(crate) fn render_queue(&mut self) -> &mut VecDeque<u8> {
        &mut self.bytes
    }

    pub(crate) fn push_capture(&mut self, packet: &[u8]) {
        debug_assert!(packet.len().is_multiple_of(FRAME_BYTES));
        debug_assert!(packet.len() <= MAX_CAPTURE_FRAMES * FRAME_BYTES);
        let overflow =
            (self.bytes.len() + packet.len()).saturating_sub(self.queue_limit * FRAME_BYTES);
        if overflow != 0 {
            self.reset_drift();
        }
        self.bytes.drain(..overflow);
        self.bytes.extend(packet);
    }

    pub(crate) fn reset_drift(&mut self) {
        self.drift = DriftTracker::default();
        self.reserve_correction = ReserveCorrection::default();
    }

    pub(crate) fn observe_clocks(
        &mut self,
        position: u64,
        timestamp: u64,
        sample_render: impl FnOnce() -> Option<ClockPosition>,
    ) {
        let capture = ClockPosition::new(position, timestamp, self.sample_rate as u64);
        self.drift.observe(capture, sample_render);
    }

    pub(crate) fn balance(&mut self, padding: usize, captured_frames: usize) {
        let queued = self.queued_frames();
        let target = self.render_capacity + self.reserve;
        if padding == 0 {
            self.reset_drift();
            if queued == 0 {
                return;
            }

            // After a complete underrun, rebuild the reserve without dropping queued audio.
            let missing_bytes = target.saturating_sub(queued) * FRAME_BYTES;
            self.bytes.extend(std::iter::repeat_n(0, missing_bytes));
            self.bytes.rotate_right(missing_bytes);
            self.reserve_correction.baseline = Some(target);
            return;
        }

        if captured_frames < 2 {
            return;
        }
        let correction = if let Some(correction) = self.drift.correction(captured_frames) {
            self.reserve_correction.clear_pending();
            correction
        } else {
            self.reserve_correction.correct(
                queued + padding,
                target,
                self.reserve,
                captured_frames,
                self.sample_rate * 2,
            )
        };
        match correction {
            1 if queued < self.queue_limit => self.splice_tail(true),
            -1 => self.splice_tail(false),
            _ => {}
        }
    }

    fn splice_tail(&mut self, insert: bool) {
        let start = self.bytes.len() - 2 * FRAME_BYTES;
        let mut a = [0; FRAME_BYTES];
        let mut b = [0; FRAME_BYTES];
        for (out, byte) in a.iter_mut().chain(&mut b).zip(self.bytes.range(start..)) {
            *out = *byte;
        }

        // Adjust one complete stereo frame, interpolating instead of inserting a zero or a hard cut.
        let mut midpoint = [0; FRAME_BYTES];
        for offset in [0, 4] {
            let left = f32::from_le_bytes(a[offset..offset + 4].try_into().unwrap());
            let right = f32::from_le_bytes(b[offset..offset + 4].try_into().unwrap());
            midpoint[offset..offset + 4].copy_from_slice(&(left * 0.5 + right * 0.5).to_le_bytes());
        }
        self.bytes.truncate(start);
        if insert {
            self.bytes.extend(a);
        }
        self.bytes.extend(midpoint);
        if insert {
            self.bytes.extend(b);
        }
    }
}

#[derive(Default)]
struct ReserveCorrection {
    baseline: Option<usize>,
    direction: i8,
    outside_frames: usize,
    credit: usize,
}

impl ReserveCorrection {
    fn clear_pending(&mut self) {
        self.direction = 0;
        self.outside_frames = 0;
        self.credit = 0;
    }

    fn correct(
        &mut self,
        fill: usize,
        target: usize,
        reserve: usize,
        captured: usize,
        confirmation_frames: usize,
    ) -> i8 {
        let baseline = *self
            .baseline
            .get_or_insert_with(|| fill.clamp(target - reserve * 3 / 4, target));
        let displacement = fill as isize - baseline as isize;
        if self.outside_frames >= confirmation_frames {
            if displacement * isize::from(self.direction) >= -(reserve as isize / 4) {
                self.clear_pending();
                return 0;
            }
        } else {
            if displacement.unsigned_abs() <= reserve / 2 {
                self.clear_pending();
                return 0;
            }
            let direction = -displacement.signum() as i8;
            if direction != self.direction {
                self.clear_pending();
                self.direction = direction;
            }
            self.outside_frames += captured;
            if self.outside_frames < confirmation_frames {
                return 0;
            }
        }

        const FRAMES_PER_CORRECTION: usize = 4_000;
        self.credit += captured;
        if self.credit < FRAMES_PER_CORRECTION {
            return 0;
        }
        self.credit -= FRAMES_PER_CORRECTION;
        self.direction
    }
}

#[cfg(test)]
#[path = "relay_buffer_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "relay_clock_tests.rs"]
mod clock_tests;
