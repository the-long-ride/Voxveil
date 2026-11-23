use std::time::Duration;

pub(super) struct Prefetch {
    pub rate: u32,
    pub total: Option<u64>,
    context: usize,
    frames: u64,
    work: Duration,
    rtf: f64,
    measured: bool,
    active: Option<Duration>,
}
impl Prefetch {
    pub fn new(rate: u32, total: Option<u64>, context: usize) -> Self {
        Self {
            rate,
            total,
            context,
            frames: 0,
            work: Duration::ZERO,
            rtf: 0.0,
            measured: false,
            active: None,
        }
    }
    pub fn start(&mut self, now: Duration) {
        self.active = Some(now);
    }
    pub fn finish(&mut self, now: Duration, frames: usize) {
        self.work += now.saturating_sub(self.active.take().unwrap_or(now));
        self.frames += frames as u64;
        if self.frames >= self.context.max(1) as u64 {
            self.measured = true;
            self.rtf = self
                .rtf
                .max(self.work.as_secs_f64() * self.rate as f64 / self.frames as f64);
        }
    }
    pub fn wait_ms(&self, now: Duration, written: u64, finished: bool) -> Option<u64> {
        if finished {
            return Some(0);
        }
        if !self.measured {
            return None;
        }
        let warmup = (self.context.saturating_mul(2) as u64).saturating_sub(self.frames) as f64
            / self.rate as f64
            * self.rtf;
        if self.rtf <= 0.9 {
            return Some((warmup * 1000.0).ceil() as u64);
        }
        let remaining = self.total?.saturating_sub(written) as f64 / self.rate as f64;
        let buffered = written as f64 / self.rate as f64;
        let rtf = self.rtf * 1.1;
        let burst = rtf * remaining.min(self.context as f64 / self.rate as f64);
        let credit = self.active.map_or(0.0, |start| {
            now.saturating_sub(start).as_secs_f64().min(burst)
        });
        let wait = ((rtf - 1.0).max(0.0) * remaining + burst + 0.25 - buffered - credit).max(0.0);
        Some((wait.max(warmup) * 1000.0).ceil() as u64)
    }
}
