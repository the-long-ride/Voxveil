use crate::relay_buffer::DEVICE_BUFFER_MS;
use std::time::{Duration, Instant};

pub(crate) struct IdleCapture {
    last_packet: Instant,
    filled: bool,
}

impl IdleCapture {
    pub fn new(now: Instant) -> Self {
        Self {
            last_packet: now,
            filled: false,
        }
    }

    pub fn captured(&mut self, now: Instant) -> bool {
        self.last_packet = now;
        std::mem::take(&mut self.filled)
    }

    pub fn frames_due(&mut self, now: Instant, drained: bool, needed: usize) -> usize {
        if !drained
            || now.saturating_duration_since(self.last_packet)
                < Duration::from_millis(DEVICE_BUFFER_MS as u64)
        {
            return 0;
        }
        self.filled |= needed != 0;
        needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waits_for_packet_cadence_and_rebases_only_after_synthesizing_silence() {
        let start = Instant::now();
        let mut idle = IdleCapture::new(start);
        assert_eq!(
            idle.frames_due(start + Duration::from_millis(99), true, 8),
            0
        );
        assert_eq!(idle.frames_due(start + Duration::from_secs(1), false, 8), 0);
        assert!(!idle.captured(start));
        assert_eq!(idle.frames_due(start + Duration::from_secs(1), true, 0), 0);
        assert!(!idle.captured(start));
        assert_eq!(idle.frames_due(start + Duration::from_secs(1), true, 8), 8);
        assert!(idle.captured(start + Duration::from_secs(1)));
        assert!(!idle.captured(start + Duration::from_secs(1)));
    }
}
