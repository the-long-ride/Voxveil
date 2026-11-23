const SAMPLE_INTERVAL_HNS: u64 = 10_000_000;
const WINDOW_HNS: u64 = 10 * SAMPLE_INTERVAL_HNS;
const DRIFT_DEADBAND: f64 = 75.0e-6;
const MAX_DRIFT: f64 = 5_000.0e-6;

#[derive(Clone, Copy)]
pub(crate) struct ClockPosition {
    position: u64,
    timestamp: u64,
    frequency: u64,
}

impl ClockPosition {
    pub(crate) fn new(position: u64, timestamp: u64, frequency: u64) -> Self {
        Self {
            position,
            timestamp,
            frequency,
        }
    }
}

#[derive(Default)]
pub(super) struct DriftTracker {
    samples: Vec<(ClockPosition, ClockPosition)>,
    previous_rate: f64,
    correction_rate: f64,
    credit: f64,
}

impl DriftTracker {
    pub(super) fn observe(
        &mut self,
        capture: ClockPosition,
        sample_render: impl FnOnce() -> Option<ClockPosition>,
    ) {
        if capture.timestamp == 0 {
            *self = Self::default();
            return;
        }
        if let Some((last_capture, _)) = self.samples.last() {
            if capture.timestamp < last_capture.timestamp {
                *self = Self::default();
            } else if capture.timestamp - last_capture.timestamp < SAMPLE_INTERVAL_HNS {
                return;
            }
        }
        let Some(render) = sample_render().filter(|sample| sample.timestamp != 0) else {
            *self = Self::default();
            return;
        };
        if self
            .samples
            .last()
            .is_some_and(|(last_capture, last_render)| {
                capture.position < last_capture.position
                    || render.position < last_render.position
                    || render.timestamp <= last_render.timestamp
            })
        {
            *self = Self::default();
        }
        self.samples.push((capture, render));
        if capture.timestamp - self.samples[0].0.timestamp < WINDOW_HNS {
            return;
        }

        let capture_rate = fitted_rate(self.samples.iter().map(|&(capture, _)| capture));
        let render_rate = fitted_rate(self.samples.iter().map(|&(_, render)| render));
        self.samples.clear();
        self.samples.push((capture, render));
        let rate = render_rate / capture_rate - 1.0;
        if !(DRIFT_DEADBAND..=MAX_DRIFT).contains(&rate.abs()) {
            self.previous_rate = 0.0;
            self.correction_rate = 0.0;
            self.credit = 0.0;
        } else if rate * self.previous_rate <= 0.0 {
            self.previous_rate = rate;
            self.correction_rate = 0.0;
            self.credit = 0.0;
        } else {
            self.correction_rate = (self.previous_rate + rate) * 0.5;
            self.previous_rate = rate;
        }
    }

    pub(super) fn correction(&mut self, frames: usize) -> i8 {
        self.credit += self.correction_rate * frames as f64;
        if self.credit.abs() < 1.0 {
            return 0;
        }
        let step = self.credit.signum();
        self.credit -= step;
        step as i8
    }
}

fn fitted_rate(mut samples: impl Iterator<Item = ClockPosition>) -> f64 {
    let first = samples.next().unwrap();
    let (mut n, mut x, mut y, mut xx, mut xy) = (1.0, 0.0, 0.0, 0.0, 0.0);
    for sample in samples {
        let time = (sample.timestamp - first.timestamp) as f64 / SAMPLE_INTERVAL_HNS as f64;
        let position = (sample.position - first.position) as f64 / first.frequency as f64;
        n += 1.0;
        x += time;
        y += position;
        xx += time * time;
        xy += time * position;
    }
    (n * xy - x * y) / (n * xx - x * x)
}

#[cfg(windows)]
pub(crate) struct RenderClock(Option<(wasapi::AudioClock, u64)>);

#[cfg(windows)]
impl RenderClock {
    pub(crate) fn new(client: &wasapi::AudioClient) -> Self {
        Self(client.get_audioclock().ok().and_then(|clock| {
            let frequency = clock
                .get_frequency()
                .ok()
                .filter(|&frequency| frequency != 0)?;
            Some((clock, frequency))
        }))
    }

    pub(crate) fn position(&self) -> Option<ClockPosition> {
        let (clock, frequency) = self.0.as_ref()?;
        let (position, timestamp) = clock.get_position().ok()?;
        Some(ClockPosition::new(position, timestamp, *frequency))
    }
}
