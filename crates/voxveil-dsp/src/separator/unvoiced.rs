use super::math::ramp;
use super::params::SeparatorParams;

/// Detects sibilance/breath: noise-like mid energy in 4-16 kHz, without a low-frequency
/// attack, near a sung phrase (`presence`) while the voice is momentarily unvoiced.
pub struct UnvoicedDetector {
    start: usize,
    end: usize,
    flatness_lo: f32,
    flatness_hi: f32,
}

impl UnvoicedDetector {
    pub fn new(bins: usize, bin_hz: f32, params: &SeparatorParams) -> Self {
        let bin = |hz: f32| ((hz / bin_hz).round() as usize).min(bins - 1);
        Self {
            start: bin(params.unvoiced_lo_hz),
            end: bin(params.unvoiced_hi_hz),
            flatness_lo: params.flatness_lo,
            flatness_hi: params.flatness_hi,
        }
    }

    pub fn update(
        &self,
        mid_magnitude: &[f32],
        low_flux: f32,
        presence: f32,
        voicing: f32,
        out: &mut [f32],
    ) -> f32 {
        out.fill(0.0);
        if self.end <= self.start {
            return 0.0;
        }
        let (mut log_sum, mut sum) = (0.0_f64, 0.0_f64);
        for &m in &mid_magnitude[self.start..=self.end] {
            let power = f64::from(m * m) + 1.0e-12;
            log_sum += power.ln();
            sum += power;
        }
        let count = (self.end - self.start + 1) as f64;
        let flatness = ((log_sum / count).exp() / (sum / count)) as f32;
        let gate = ramp(flatness, self.flatness_lo, self.flatness_hi)
            * (1.0 - low_flux).clamp(0.0, 1.0)
            * presence.clamp(0.0, 1.0)
            * (1.0 - voicing).clamp(0.0, 1.0);
        out[self.start..=self.end].fill(gate);
        gate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIN_HZ: f32 = 48_000.0 / 4_096.0;

    fn noise_band() -> Vec<f32> {
        let mut state = 5_u32;
        (0..2_049)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                // Rayleigh-like magnitudes: sqrt of exponential power.
                let u = ((state >> 8) as f32 + 1.0) / (1_u32 << 24) as f32;
                (-u.ln()).sqrt()
            })
            .collect()
    }

    fn tonal_band() -> Vec<f32> {
        (0..2_049).map(|k| if k % 40 == 0 { 10.0 } else { 0.001 }).collect()
    }

    fn gate(magnitude: &[f32], low_flux: f32, presence: f32, voicing: f32) -> (f32, Vec<f32>) {
        let detector = UnvoicedDetector::new(2_049, BIN_HZ, &SeparatorParams::default());
        let mut out = vec![0.0; 2_049];
        let value = detector.update(magnitude, low_flux, presence, voicing, &mut out);
        (value, out)
    }

    #[test]
    fn noise_near_a_sung_phrase_is_unvoiced_vocal() {
        let (value, out) = gate(&noise_band(), 0.0, 1.0, 0.0);
        assert!(value > 0.6, "gate={value}");
        let band = (4_000.0 / BIN_HZ).round() as usize;
        assert_eq!(out[band], value);
        assert_eq!(out[band / 2], 0.0);
    }

    #[test]
    fn tonal_content_drum_attacks_sustained_notes_and_silence_of_voice_are_spared() {
        assert!(gate(&tonal_band(), 0.0, 1.0, 0.0).0 < 0.1);
        assert_eq!(gate(&noise_band(), 1.0, 1.0, 0.0).0, 0.0);
        assert_eq!(gate(&noise_band(), 0.0, 1.0, 1.0).0, 0.0);
        assert_eq!(gate(&noise_band(), 0.0, 0.0, 0.0).0, 0.0);
    }
}
