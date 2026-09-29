use super::fft::Complex32;
use super::math::one_pole_coeff;
use super::params::SeparatorParams;

const EPSILON: f32 = 1.0e-12;
const SIDE_RATIO_SECONDS: f32 = 0.1;

/// Per-bin probability that energy is panned to the center, from recursively averaged
/// cross-spectra (similarity and phase agreement), plus a frame-level near-mono guard.
pub struct CenterEstimator {
    avg: f32,
    psi_exp: f32,
    rho_exp: f32,
    phi_ll: Box<[f32]>,
    phi_rr: Box<[f32]>,
    phi_lr: Box<[Complex32]>,
    raw: Box<[f32]>,
    side_ratio: f32,
    side_coeff: f32,
}

impl CenterEstimator {
    pub fn new(bins: usize, hop_seconds: f32, params: &SeparatorParams) -> Self {
        Self {
            avg: params.center_avg.clamp(0.0, 0.99),
            psi_exp: params.center_psi_exp.max(0.1),
            rho_exp: params.center_rho_exp.max(0.1),
            phi_ll: vec![0.0; bins].into_boxed_slice(),
            phi_rr: vec![0.0; bins].into_boxed_slice(),
            phi_lr: vec![Complex32::ZERO; bins].into_boxed_slice(),
            raw: vec![0.0; bins].into_boxed_slice(),
            side_ratio: 1.0,
            side_coeff: one_pole_coeff(hop_seconds, SIDE_RATIO_SECONDS),
        }
    }

    pub fn reset(&mut self) {
        self.phi_ll.fill(0.0);
        self.phi_rr.fill(0.0);
        self.phi_lr.fill(Complex32::ZERO);
        self.side_ratio = 1.0;
    }

    /// 1.0 when side energy is at or below 0.001 of mid energy, 0.0 at or above 0.01.
    pub fn mono_confidence(&self) -> f32 {
        1.0 - ((self.side_ratio - 0.001) / 0.009).clamp(0.0, 1.0)
    }

    pub fn update(&mut self, left: &[Complex32], right: &[Complex32], center: &mut [f32]) {
        let a = self.avg;
        let b = 1.0 - a;
        let (mut mid_total, mut side_total) = (0.0_f64, 0.0_f64);
        for k in 0..self.raw.len() {
            let (l, r) = (left[k], right[k]);
            self.phi_ll[k] = a * self.phi_ll[k] + b * l.norm_sqr();
            self.phi_rr[k] = a * self.phi_rr[k] + b * r.norm_sqr();
            self.phi_lr[k] = self.phi_lr[k].scale(a).add(l.mul(r.conj()).scale(b));
            let cross = self.phi_lr[k].magnitude();
            let psi = (2.0 * cross / (self.phi_ll[k] + self.phi_rr[k] + EPSILON)).clamp(0.0, 1.0);
            let rho = (self.phi_lr[k].re / (cross + EPSILON)).max(0.0);
            self.raw[k] = psi.powf(self.psi_exp) * rho.powf(self.rho_exp);
            mid_total += f64::from(l.add(r).scale(0.5).norm_sqr());
            side_total += f64::from(l.sub(r).scale(0.5).norm_sqr());
        }
        if mid_total > 1.0e-12 {
            let ratio = (side_total / mid_total).min(1.0) as f32;
            self.side_ratio += self.side_coeff * (ratio - self.side_ratio);
        }
        let last = self.raw.len() - 1;
        for k in 0..=last {
            let previous = self.raw[k.saturating_sub(1)];
            let next = self.raw[(k + 1).min(last)];
            center[k] = 0.25 * previous + 0.5 * self.raw[k] + 0.25 * next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BINS: usize = 64;

    fn run(left: &[Complex32], right: &[Complex32], hops: usize) -> (Vec<f32>, CenterEstimator) {
        let mut estimator = CenterEstimator::new(BINS, 0.021, &SeparatorParams::default());
        let mut center = vec![0.0; BINS];
        for _ in 0..hops {
            estimator.update(left, right, &mut center);
        }
        (center, estimator)
    }

    fn uniform(value: Complex32) -> Vec<Complex32> {
        vec![value; BINS]
    }

    #[test]
    fn identical_channels_are_center() {
        let bins = uniform(Complex32::new(1.0, 0.5));
        let (center, estimator) = run(&bins, &bins, 60);
        assert!(center.iter().all(|&c| c > 0.9), "{center:?}");
        assert!(estimator.mono_confidence() > 0.99);
    }

    #[test]
    fn hard_panned_and_anti_phase_are_not_center() {
        let left = uniform(Complex32::new(1.0, 0.5));
        let (panned, _) = run(&left, &uniform(Complex32::ZERO), 10);
        assert!(panned.iter().all(|&c| c < 0.05));
        let right: Vec<Complex32> = left.iter().map(|v| v.scale(-1.0)).collect();
        let (anti, estimator) = run(&left, &right, 30);
        assert!(anti.iter().all(|&c| c < 0.01));
        assert!(estimator.mono_confidence() < 0.01);
    }

    #[test]
    fn half_level_right_channel_is_partially_center() {
        let left = uniform(Complex32::new(1.0, 0.0));
        let right = uniform(Complex32::new(0.5, 0.0));
        let (center, _) = run(&left, &right, 30);
        // psi = 2*0.5/(1+0.25) = 0.8, c = 0.8^4 = 0.41
        assert!(center.iter().all(|&c| (0.3..0.5).contains(&c)), "{center:?}");
    }

    #[test]
    fn independent_noise_is_mostly_not_center() {
        let mut state = 11_u32;
        let mut next = move || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) as f32 / (1_u32 << 24) as f32 - 0.5
        };
        let mut estimator = CenterEstimator::new(BINS, 0.021, &SeparatorParams::default());
        let mut center = vec![0.0; BINS];
        for _ in 0..40 {
            let left: Vec<Complex32> = (0..BINS).map(|_| Complex32::new(next(), next())).collect();
            let right: Vec<Complex32> = (0..BINS).map(|_| Complex32::new(next(), next())).collect();
            estimator.update(&left, &right, &mut center);
        }
        let mean = center.iter().sum::<f32>() / BINS as f32;
        assert!(mean < 0.2, "mean={mean}");
    }
}
