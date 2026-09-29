use super::math::{one_pole_coeff, ramp};
use super::params::SeparatorParams;

pub struct MaskInputs<'a> {
    pub center: &'a [f32],
    pub comb: &'a [f32],
    pub unvoiced: &'a [f32],
    pub voicing: f32,
    pub mono_confidence: f32,
    pub broad_flux: f32,
    pub depth: f32,
}

/// Turns per-bin vocal evidence into smoothed mid gains (spec section 5.5).
pub struct MaskBuilder {
    center_share: f32,
    transient_guard: f32,
    floor: f32,
    attack: f32,
    release: f32,
    wiener_exp: f32,
    low_weight: Box<[f32]>,
    share_weight: Box<[f32]>,
    vocalness: Box<[f32]>,
    gain: Box<[f32]>,
}

impl MaskBuilder {
    pub fn new(bins: usize, bin_hz: f32, hop_seconds: f32, params: &SeparatorParams) -> Self {
        let by_bin = |low: f32, high: f32| -> Box<[f32]> {
            (0..bins).map(|k| ramp(k as f32 * bin_hz, low, high)).collect()
        };
        Self {
            center_share: params.center_share.clamp(0.0, 1.0),
            transient_guard: params.transient_guard.clamp(0.0, 1.0),
            floor: params.floor_gain.clamp(1.0e-4, 1.0),
            attack: params.attack.clamp(0.0, 1.0),
            release: one_pole_coeff(hop_seconds, params.release_s),
            wiener_exp: params.wiener_exp.max(0.1),
            low_weight: by_bin(params.low_protect_hz, params.low_full_hz),
            share_weight: by_bin(params.share_lo_hz, params.share_hi_hz),
            vocalness: vec![0.0; bins].into_boxed_slice(),
            gain: vec![1.0; bins].into_boxed_slice(),
        }
    }

    pub fn reset(&mut self) {
        self.gain.fill(1.0);
    }

    pub fn gains(&self) -> &[f32] {
        &self.gain
    }

    pub fn build(&mut self, inputs: &MaskInputs<'_>) -> &[f32] {
        let frame_share = self.center_share
            * (1.0 - inputs.mono_confidence).clamp(0.0, 1.0)
            * (1.0 - self.transient_guard * inputs.broad_flux).clamp(0.0, 1.0);
        for k in 0..self.gain.len() {
            let share = frame_share * self.share_weight[k];
            let pitched = (inputs.comb[k] * inputs.voicing).max(inputs.unvoiced[k]);
            self.vocalness[k] = inputs.center[k] * (share + (1.0 - share) * pitched);
        }
        let last = self.gain.len() - 1;
        for k in 0..=last {
            // Hole filling keeps harmonic peaks at full depth and removes isolated dropouts.
            let neighbours =
                0.5 * (self.vocalness[k.saturating_sub(1)] + self.vocalness[(k + 1).min(last)]);
            let p = self.vocalness[k].max(neighbours) * self.low_weight[k];
            let target = (1.0 - inputs.depth * p)
                .clamp(0.0, 1.0)
                .powf(self.wiener_exp)
                .max(self.floor);
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

    struct Case {
        center: Vec<f32>,
        comb: Vec<f32>,
        unvoiced: Vec<f32>,
        voicing: f32,
        mono: f32,
        depth: f32,
    }

    impl Case {
        fn new() -> Self {
            Self {
                center: vec![0.0; BINS],
                comb: vec![0.0; BINS],
                unvoiced: vec![0.0; BINS],
                voicing: 0.0,
                mono: 0.0,
                depth: 1.0,
            }
        }

        fn run(&self, mask: &mut MaskBuilder, hops: usize) -> Vec<f32> {
            let inputs = MaskInputs {
                center: &self.center,
                comb: &self.comb,
                unvoiced: &self.unvoiced,
                voicing: self.voicing,
                mono_confidence: self.mono,
                broad_flux: 0.0,
                depth: self.depth,
            };
            for _ in 0..hops {
                mask.build(&inputs);
            }
            mask.gains().to_vec()
        }
    }

    fn builder() -> MaskBuilder {
        MaskBuilder::new(BINS, BIN_HZ, 0.021, &SeparatorParams::default())
    }

    fn bin(hz: f32) -> usize {
        (hz / BIN_HZ).round() as usize
    }

    #[test]
    fn no_vocal_evidence_keeps_unity() {
        let gains = Case::new().run(&mut builder(), 10);
        assert!(gains.iter().all(|&g| g == 1.0));
    }

    #[test]
    fn centered_voiced_harmonic_reaches_the_floor_but_low_bass_is_protected() {
        let mut case = Case::new();
        case.center.fill(1.0);
        case.voicing = 1.0;
        case.comb[bin(1_000.0)] = 1.0;
        case.comb[bin(40.0)] = 1.0;
        let gains = case.run(&mut builder(), 10);
        assert!(gains[bin(1_000.0)] <= 0.0101, "{}", gains[bin(1_000.0)]);
        assert_eq!(gains[bin(40.0)], 1.0);
    }

    #[test]
    fn center_only_share_cuts_mids_spares_bass_harmonics_and_mono() {
        let mut case = Case::new();
        case.center.fill(1.0);
        let gains = case.run(&mut builder(), 20);
        assert!((gains[bin(1_000.0)] - 0.65).abs() < 0.01, "{}", gains[bin(1_000.0)]);
        assert_eq!(gains[bin(150.0)], 1.0);
        case.mono = 1.0;
        let mono = case.run(&mut builder(), 20);
        assert_eq!(mono[bin(1_000.0)], 1.0);
    }

    #[test]
    fn zero_depth_is_unity_and_release_is_gradual() {
        let mut case = Case::new();
        case.center.fill(1.0);
        case.voicing = 1.0;
        case.comb.fill(1.0);
        case.depth = 0.0;
        assert!(case.run(&mut builder(), 5).iter().all(|&g| g == 1.0));

        let mut mask = builder();
        case.depth = 1.0;
        case.run(&mut mask, 10);
        let released = Case::new().run(&mut mask, 1);
        assert!(released[bin(1_000.0)] < 0.5, "{}", released[bin(1_000.0)]);
        mask.reset();
        assert!(mask.gains().iter().all(|&g| g == 1.0));
    }
}
