use std::collections::VecDeque;

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
    }

    pub(crate) fn balance(&mut self, padding: usize, captured_frames: usize) {
        if padding == 0 {
            self.reset_drift();
        }
        if captured_frames == 0 {
            return;
        }

        let queued = self.queued_frames();
        if padding == 0 && queued < self.render_capacity {
            // After a complete underrun, rebuild the reserve without dropping queued audio.
            let target = self.render_capacity + self.reserve;
            self.bytes
                .extend(std::iter::repeat_n(0, (target - queued) * FRAME_BYTES));
            self.bytes.rotate_right((target - queued) * FRAME_BYTES);
            return;
        }

        if captured_frames < 2 {
            return;
        }
        let total = queued + padding;
        let deadband = (self.sample_rate * 12 / 1_000)
            .min(self.reserve * 3 / 4)
            .max(1);
        match self
            .drift
            .observe(total, captured_frames, self.sample_rate, deadband)
        {
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
struct DriftTracker {
    weighted_total: u64,
    frames: usize,
    baseline: Option<f64>,
    previous_average: f64,
    trend: i8,
    adjustment: i8,
}

impl DriftTracker {
    fn observe(&mut self, total: usize, frames: usize, sample_rate: usize, deadband: usize) -> i8 {
        self.weighted_total += total as u64 * frames as u64;
        self.frames += frames;
        if self.frames >= sample_rate * 2 {
            let average = self.weighted_total as f64 / self.frames as f64;
            self.frames = 0;
            self.weighted_total = 0;

            if self.adjustment == 0
                && (average - self.previous_average).abs() > sample_rate as f64 / 200.0
            {
                // A step of more than 5 ms in a window is a new occupancy baseline, not drift.
                self.baseline = Some(average);
                self.trend = 0;
            } else if let Some(baseline) = self.baseline {
                let step = match average.total_cmp(&self.previous_average) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                self.trend = if step == self.trend.signum() {
                    self.trend.saturating_add(step)
                } else {
                    step
                };
                let error = average - baseline;
                if self.adjustment != 0 {
                    if error * f64::from(self.adjustment) >= -(deadband as f64 / 4.0) {
                        self.adjustment = 0;
                        self.trend = 0;
                    }
                } else if error.abs() > deadband as f64
                    && error * f64::from(self.trend) > 0.0
                    && self.trend.unsigned_abs() >= 2
                {
                    self.adjustment = -self.trend.signum();
                }
            } else {
                self.baseline = Some(average);
            }
            self.previous_average = average;
        }

        let Some(baseline) = self.baseline else {
            return 0;
        };
        if (total as f64 - baseline) * f64::from(self.adjustment) < 0.0 {
            self.adjustment
        } else {
            0
        }
    }
}

#[cfg(test)]
#[path = "relay_buffer_tests.rs"]
mod tests;
