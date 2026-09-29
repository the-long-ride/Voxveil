const LOW_MIN_HZ: f32 = 40.0;
const LOW_MAX_HZ: f32 = 300.0;
const EPSILON: f32 = 1.0e-3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FluxReading {
    pub low: f32,
    pub broad: f32,
}

/// Net energy-onset detector on the mid magnitude spectrum.
pub struct SpectralFlux {
    previous: Box<[f32]>,
    low_start: usize,
    low_end: usize,
}

impl SpectralFlux {
    pub fn new(bins: usize, bin_hz: f32) -> Self {
        let bin = |hz: f32| ((hz / bin_hz).round() as usize).clamp(1, bins - 1);
        Self {
            previous: vec![0.0; bins].into_boxed_slice(),
            low_start: bin(LOW_MIN_HZ),
            low_end: bin(LOW_MAX_HZ),
        }
    }

    pub fn reset(&mut self) {
        self.previous.fill(0.0);
    }

    pub fn update(&mut self, magnitude: &[f32]) -> FluxReading {
        let (mut low_now, mut low_before, mut now, mut before) = (0.0, 0.0, 0.0, 0.0);
        for (k, (&current, previous)) in magnitude.iter().zip(self.previous.iter_mut()).enumerate() {
            now += current;
            before += *previous;
            if (self.low_start..=self.low_end).contains(&k) {
                low_now += current;
                low_before += *previous;
            }
            *previous = current;
        }
        FluxReading { low: onset(low_now, low_before), broad: onset(now, before) }
    }
}

fn onset(now: f32, before: f32) -> f32 {
    let ratio = (now - before).max(0.0) / (before + EPSILON);
    ratio / (ratio + 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIN_HZ: f32 = 48_000.0 / 4_096.0;

    #[test]
    fn steady_spectrum_has_no_onset() {
        let mut flux = SpectralFlux::new(2_049, BIN_HZ);
        let spectrum = vec![1.0; 2_049];
        flux.update(&spectrum);
        let reading = flux.update(&spectrum);
        assert!(reading.low < 0.01 && reading.broad < 0.01, "{reading:?}");
    }

    #[test]
    fn step_from_silence_is_a_strong_onset_and_decay_is_not() {
        let mut flux = SpectralFlux::new(2_049, BIN_HZ);
        flux.update(&vec![0.0; 2_049]);
        let onset = flux.update(&vec![1.0; 2_049]);
        assert!(onset.low > 0.9 && onset.broad > 0.9, "{onset:?}");
        let decay = flux.update(&vec![0.5; 2_049]);
        assert_eq!(decay, FluxReading { low: 0.0, broad: 0.0 });
    }

    #[test]
    fn high_band_onset_does_not_raise_low_flux() {
        let mut flux = SpectralFlux::new(2_049, BIN_HZ);
        let mut spectrum = vec![1.0; 2_049];
        flux.update(&spectrum);
        for value in &mut spectrum[400..] {
            *value = 5.0;
        }
        let reading = flux.update(&spectrum);
        assert!(reading.low < 0.01 && reading.broad > 0.5, "{reading:?}");
    }
}
