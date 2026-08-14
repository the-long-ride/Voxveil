use voxveil_audio_core::AudioProcessor;
use voxveil_types::VocalLevel;

pub struct MidSideSuppressor {
    vocal_level: VocalLevel,
}

impl MidSideSuppressor {
    pub fn new(vocal_level: VocalLevel) -> Self {
        Self { vocal_level }
    }

    pub fn set_vocal_level(&mut self, level: VocalLevel) {
        self.vocal_level = level;
    }

    fn process_pair(&self, left: f32, right: f32) -> (f32, f32) {
        let mid = (left + right) * 0.5;
        let side = (left - right) * 0.5;
        let reduced_mid = mid * self.vocal_level.get();
        (reduced_mid + side, reduced_mid - side)
    }
}

impl AudioProcessor for MidSideSuppressor {
    fn process_stereo_interleaved(&mut self, samples: &mut [f32]) {
        for pair in samples.chunks_exact_mut(2) {
            let (left, right) = self.process_pair(pair[0], pair[1]);
            pair[0] = left;
            pair[1] = right;
        }
    }

    fn latency_frames(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(value: f32) -> VocalLevel {
        VocalLevel::new(value).unwrap()
    }

    #[test]
    fn full_vocal_level_preserves_input() {
        let mut dsp = MidSideSuppressor::new(level(1.0));
        let mut samples = [0.25, -0.5, 0.7, 0.7];
        let original = samples;
        dsp.process_stereo_interleaved(&mut samples);
        assert_eq!(samples, original);
    }

    #[test]
    fn zero_vocal_level_removes_fully_centered_signal() {
        let mut dsp = MidSideSuppressor::new(level(0.0));
        let mut samples = [0.8, 0.8, -0.4, -0.4];
        dsp.process_stereo_interleaved(&mut samples);
        assert!(samples.iter().all(|value| value.abs() < 1e-6));
    }

    #[test]
    fn pure_side_information_is_preserved() {
        let mut dsp = MidSideSuppressor::new(level(0.0));
        let mut samples = [0.6, -0.6];
        dsp.process_stereo_interleaved(&mut samples);
        assert!((samples[0] - 0.6).abs() < 1e-6);
        assert!((samples[1] + 0.6).abs() < 1e-6);
    }

    #[test]
    fn finite_input_stays_finite() {
        let mut dsp = MidSideSuppressor::new(level(0.25));
        let mut samples = [1.0, -1.0, 0.5, 0.25];
        dsp.process_stereo_interleaved(&mut samples);
        assert!(samples.iter().all(|value| value.is_finite()));
    }
}
