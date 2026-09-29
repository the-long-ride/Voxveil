use super::math::one_pole_coeff;
use super::params::SeparatorParams;

/// Attenuates side-channel energy that decays in step with recently removed center
/// energy (vocal reverb, delays, doubles), inside a vocal band only.
pub struct SideReverbSuppressor {
    decay: f32,
    beta: f32,
    floor: f32,
    attack: f32,
    release: f32,
    start: usize,
    end: usize,
    memory: Box<[f32]>,
    gain: Box<[f32]>,
}

impl SideReverbSuppressor {
    pub fn new(bins: usize, bin_hz: f32, hop_seconds: f32, params: &SeparatorParams) -> Self {
        let bin = |hz: f32| ((hz / bin_hz).round() as usize).min(bins - 1);
        Self {
            // T60 = 60 dB power decay.
            decay: 10.0_f32.powf(-6.0 * hop_seconds / params.reverb_t60_s.max(0.05)),
            beta: params.reverb_beta.max(0.0),
            floor: params.side_floor.clamp(0.0, 1.0),
            attack: params.attack.clamp(0.0, 1.0),
            release: one_pole_coeff(hop_seconds, params.release_s),
            start: bin(params.reverb_lo_hz),
            end: bin(params.reverb_hi_hz),
            memory: vec![0.0; bins].into_boxed_slice(),
            gain: vec![1.0; bins].into_boxed_slice(),
        }
    }

    pub fn reset(&mut self) {
        self.memory.fill(0.0);
        self.gain.fill(1.0);
    }

    pub fn gains(&self) -> &[f32] {
        &self.gain
    }

    pub fn update(
        &mut self,
        mid_gain: &[f32],
        mid_power: &[f32],
        side_power: &[f32],
        depth: f32,
    ) -> &[f32] {
        for k in 0..self.gain.len() {
            let removed = (1.0 - mid_gain[k] * mid_gain[k]).max(0.0) * mid_power[k];
            self.memory[k] = (self.memory[k] * self.decay).max(removed);
            let target = if (self.start..=self.end).contains(&k) {
                (1.0 - self.beta * depth * self.memory[k] / (side_power[k] + 1.0e-9))
                    .clamp(self.floor, 1.0)
            } else {
                1.0
            };
            let current = self.gain[k];
            let coeff = if target < current { self.attack } else { self.release };
            self.gain[k] = current + coeff * (target - current);
        }
        &self.gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BINS: usize = 2_049;
    const BIN_HZ: f32 = 48_000.0 / 4_096.0;
    const HOP: f32 = 1_024.0 / 48_000.0;
    const K: usize = 100; // ~1.17 kHz, inside 200 Hz..8 kHz

    fn suppressor() -> SideReverbSuppressor {
        SideReverbSuppressor::new(BINS, BIN_HZ, HOP, &SeparatorParams::default())
    }

    #[test]
    fn steady_side_content_without_removed_center_is_untouched() {
        let mut reverb = suppressor();
        let unity = vec![1.0; BINS];
        let zero = vec![0.0; BINS];
        let side = vec![1.0; BINS];
        for _ in 0..20 {
            reverb.update(&unity, &zero, &side, 1.0);
        }
        assert!(reverb.gains().iter().all(|&g| (g - 1.0).abs() < 0.06));
    }

    #[test]
    fn side_tail_after_removed_center_is_reduced() {
        let mut reverb = suppressor();
        let cut = vec![0.01; BINS];
        let mut mid_power = vec![0.0; BINS];
        let mut side_power = vec![0.0; BINS];
        mid_power[K] = 1.0;
        for _ in 0..10 {
            reverb.update(&cut, &mid_power, &side_power, 1.0);
        }
        mid_power[K] = 0.0;
        let mut tail = 0.1_f32;
        for _ in 0..3 {
            side_power[K] = tail;
            reverb.update(&cut, &mid_power, &side_power, 1.0);
            tail *= 0.8;
        }
        assert!(reverb.gains()[K] < 0.63, "gain={}", reverb.gains()[K]);
        assert!(reverb.gains()[K] >= 0.25);
    }

    #[test]
    fn outside_band_and_zero_depth_are_unity() {
        let mut reverb = suppressor();
        let cut = vec![0.01; BINS];
        let power = vec![1.0; BINS];
        let tiny = vec![0.001; BINS];
        for _ in 0..10 {
            reverb.update(&cut, &power, &tiny, 1.0);
        }
        assert_eq!(reverb.gains()[5], 1.0);
        assert_eq!(reverb.gains()[BINS - 1], 1.0);
        let mut fresh = suppressor();
        for _ in 0..10 {
            fresh.update(&cut, &power, &tiny, 0.0);
        }
        assert!(fresh.gains().iter().all(|&g| g == 1.0));
    }
}
