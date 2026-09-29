use super::math::{one_pole_coeff, ramp};
use super::params::SeparatorParams;

pub const MAX_HARMONICS: usize = 12;
const STEPS_PER_OCTAVE: f32 = 96.0;
const HARMONIC_DECAY: f32 = 0.84;
const TOP_CANDIDATES: usize = 3;
const TRACK_VOICING: f32 = 0.3;
const BAND_MIN_HZ: f32 = 60.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PitchEstimate {
    pub f0_hz: f32,
    pub voicing: f32,
}

/// Predominant-pitch tracker over a centered magnitude spectrum.
pub struct PitchTracker {
    candidates: Box<[f32]>,
    harmonic_bins: Box<[u16]>,
    midpoint_bins: Box<[u16]>,
    weights: [f32; MAX_HARMONICS],
    salience: Box<[f32]>,
    band_start: usize,
    band_end: usize,
    previous_f0: f32,
    voicing: f32,
    release: f32,
    jump_penalty: f32,
    octave_guard: f32,
    voicing_lo: f32,
    voicing_hi: f32,
}

impl PitchTracker {
    pub fn new(bins: usize, bin_hz: f32, hop_seconds: f32, params: &SeparatorParams) -> Self {
        let min_hz = params.pitch_min_hz.max(2.0 * bin_hz);
        let max_hz = params.pitch_max_hz.max(min_hz * 1.01);
        let steps = ((max_hz / min_hz).log2() * STEPS_PER_OCTAVE).floor() as usize + 1;
        let candidates: Box<[f32]> =
            (0..steps).map(|i| min_hz * (i as f32 / STEPS_PER_OCTAVE).exp2()).collect();
        let to_bin = |hz: f32| -> u16 {
            if hz > params.comb_max_hz {
                return 0;
            }
            let bin = (hz / bin_hz).round() as usize;
            if bin == 0 || bin + 1 >= bins || bin > usize::from(u16::MAX) { 0 } else { bin as u16 }
        };
        let mut harmonic_bins = vec![0_u16; steps * MAX_HARMONICS];
        let mut midpoint_bins = vec![0_u16; steps * MAX_HARMONICS];
        for (c, &f0) in candidates.iter().enumerate() {
            for h in 0..MAX_HARMONICS {
                harmonic_bins[c * MAX_HARMONICS + h] = to_bin(f0 * (h + 1) as f32);
                midpoint_bins[c * MAX_HARMONICS + h] = to_bin(f0 * (h as f32 + 0.5));
            }
        }
        let mut weights = [0.0; MAX_HARMONICS];
        for (h, weight) in weights.iter_mut().enumerate() {
            *weight = HARMONIC_DECAY.powi(h as i32);
        }
        let band_bin = |hz: f32| ((hz / bin_hz).round() as usize).clamp(1, bins - 2);
        Self {
            salience: vec![0.0; steps].into_boxed_slice(),
            candidates,
            harmonic_bins: harmonic_bins.into_boxed_slice(),
            midpoint_bins: midpoint_bins.into_boxed_slice(),
            weights,
            band_start: band_bin(BAND_MIN_HZ),
            band_end: band_bin(params.comb_max_hz),
            previous_f0: 0.0,
            voicing: 0.0,
            release: one_pole_coeff(hop_seconds, params.voicing_release_s),
            jump_penalty: params.pitch_jump_penalty,
            octave_guard: params.pitch_octave_guard,
            voicing_lo: params.voicing_lo,
            voicing_hi: params.voicing_hi.max(params.voicing_lo + 1.0e-3),
        }
    }

    pub fn reset(&mut self) {
        self.previous_f0 = 0.0;
        self.voicing = 0.0;
    }

    pub fn update(&mut self, magnitude: &[f32]) -> PitchEstimate {
        self.compute_salience(magnitude);
        let total: f32 = magnitude[self.band_start..=self.band_end].iter().sum();
        let picked = if total > 1.0e-9 { self.pick() } else { None };
        let raw = match picked {
            Some(index) => {
                let harmonicity = self.harmonicity(index, magnitude) / total;
                let raw = ramp(harmonicity, self.voicing_lo, self.voicing_hi);
                if raw > 0.0 {
                    self.previous_f0 = self.candidates[index];
                }
                raw
            }
            None => 0.0,
        };
        self.voicing = if raw > self.voicing {
            raw
        } else {
            self.voicing + self.release * (raw - self.voicing)
        };
        if raw == 0.0 && self.voicing < TRACK_VOICING {
            self.previous_f0 = 0.0;
        }
        PitchEstimate {
            f0_hz: self.previous_f0,
            voicing: if self.previous_f0 > 0.0 { self.voicing } else { 0.0 },
        }
    }

    fn compute_salience(&mut self, magnitude: &[f32]) {
        for (c, slot) in self.salience.iter_mut().enumerate() {
            let row = c * MAX_HARMONICS;
            let (mut harmonic, mut between) = (0.0, 0.0);
            for h in 0..MAX_HARMONICS {
                harmonic += self.weights[h] * peak(magnitude, self.harmonic_bins[row + h]);
                between += self.weights[h] * peak(magnitude, self.midpoint_bins[row + h]);
            }
            *slot = (harmonic - self.octave_guard * between).max(0.0);
        }
    }

    fn harmonicity(&self, index: usize, magnitude: &[f32]) -> f32 {
        let row = index * MAX_HARMONICS;
        (0..MAX_HARMONICS).map(|h| peak(magnitude, self.harmonic_bins[row + h])).sum()
    }

    fn pick(&self) -> Option<usize> {
        let salience = &self.salience;
        let mut top = [(0_usize, 0.0_f32); TOP_CANDIDATES];
        for c in 1..salience.len().saturating_sub(1) {
            let value = salience[c];
            if value <= 0.0 || value < salience[c - 1] || value <= salience[c + 1] {
                continue;
            }
            if let Some(slot) = top.iter().position(|&(_, t)| value > t) {
                top.copy_within(slot..TOP_CANDIDATES - 1, slot + 1);
                top[slot] = (c, value);
            }
        }
        let strongest = top[0].1;
        if strongest <= 0.0 {
            return None;
        }
        top.iter()
            .filter(|&&(_, value)| value > 0.0)
            .map(|&(c, value)| {
                let jump = if self.previous_f0 > 0.0 {
                    (12.0 * (self.candidates[c] / self.previous_f0).log2()).abs()
                } else {
                    0.0
                };
                (c, value / strongest - self.jump_penalty * jump)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(c, _)| c)
    }
}

fn peak(magnitude: &[f32], bin: u16) -> f32 {
    if bin == 0 {
        return 0.0;
    }
    let b = usize::from(bin);
    magnitude[b - 1].max(magnitude[b]).max(magnitude[b + 1])
}
