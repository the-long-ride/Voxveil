use std::collections::VecDeque;

pub(crate) const FRAME_BYTES: usize = 8;
pub(crate) const MAX_CAPTURE_FRAMES: usize = 4096;
const MAX_QUEUE_FRAMES: usize = 8192;

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
}

impl RelayBuffer {
    pub(crate) fn new(render_capacity: usize) -> Self {
        let reserve = (render_capacity / 2).clamp(1, MAX_QUEUE_FRAMES / 4);
        let mut bytes = VecDeque::with_capacity(MAX_QUEUE_FRAMES * FRAME_BYTES);
        bytes.resize(reserve * FRAME_BYTES, 0);
        Self {
            bytes,
            render_capacity,
            reserve,
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
            (self.bytes.len() + packet.len()).saturating_sub(MAX_QUEUE_FRAMES * FRAME_BYTES);
        self.bytes.drain(..overflow);
        self.bytes.extend(packet);
    }

    pub(crate) fn balance(&mut self, padding: usize, captured_frames: usize) {
        if captured_frames == 0 {
            return;
        }

        let queued = self.queued_frames();
        if padding == 0 && queued < self.render_capacity {
            // After a complete underrun, rebuild the reserve without dropping queued audio.
            let target = (self.render_capacity + self.reserve).min(MAX_QUEUE_FRAMES);
            self.bytes
                .extend(std::iter::repeat_n(0, (target - queued) * FRAME_BYTES));
            self.bytes.rotate_right((target - queued) * FRAME_BYTES);
            return;
        }

        if captured_frames < 2 {
            return;
        }
        let total = queued + padding;
        if total < self.render_capacity && queued < MAX_QUEUE_FRAMES {
            self.splice_tail(true);
        } else if total > self.render_capacity + self.reserve * 2 {
            self.splice_tail(false);
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

#[cfg(test)]
#[path = "relay_buffer_tests.rs"]
mod tests;
