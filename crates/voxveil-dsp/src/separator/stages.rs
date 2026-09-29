use super::center::CenterEstimator;
use super::comb::harmonic_comb;
use super::fft::Complex32;
use super::flux::SpectralFlux;
use super::mask::{MaskBuilder, MaskInputs};
use super::math::one_pole_coeff;
use super::params::SeparatorParams;
use super::pitch::PitchTracker;
use super::reverb::SideReverbSuppressor;
use super::unvoiced::UnvoicedDetector;

const PRESENCE_RELEASE_SECONDS: f32 = 0.3;
const DEPTH_CURVE: f32 = 0.72;

/// Every analysis stage plus preallocated scratch; `analyze` runs one hop in place.
pub struct Stages {
    bin_hz: f32,
    comb_max_hz: f32,
    vibrato_cents: f32,
    presence: f32,
    presence_release: f32,
    center: CenterEstimator,
    pitch: PitchTracker,
    flux: SpectralFlux,
    unvoiced: UnvoicedDetector,
    mask: MaskBuilder,
    reverb: SideReverbSuppressor,
    center_prob: Box<[f32]>,
    mid_magnitude: Box<[f32]>,
    centered_magnitude: Box<[f32]>,
    mid_power: Box<[f32]>,
    side_power: Box<[f32]>,
    comb: Box<[f32]>,
    unvoiced_weight: Box<[f32]>,
}

impl Stages {
    pub fn new(bins: usize, bin_hz: f32, hop_seconds: f32, params: SeparatorParams) -> Self {
        let zeros = || vec![0.0_f32; bins].into_boxed_slice();
        Self {
            bin_hz,
            comb_max_hz: params.comb_max_hz,
            vibrato_cents: params.vibrato_cents,
            presence: 0.0,
            presence_release: one_pole_coeff(hop_seconds, PRESENCE_RELEASE_SECONDS),
            center: CenterEstimator::new(bins, hop_seconds, &params),
            pitch: PitchTracker::new(bins, bin_hz, hop_seconds, &params),
            flux: SpectralFlux::new(bins, bin_hz),
            unvoiced: UnvoicedDetector::new(bins, bin_hz, &params),
            mask: MaskBuilder::new(bins, bin_hz, hop_seconds, &params),
            reverb: SideReverbSuppressor::new(bins, bin_hz, hop_seconds, &params),
            center_prob: zeros(),
            mid_magnitude: zeros(),
            centered_magnitude: zeros(),
            mid_power: zeros(),
            side_power: zeros(),
            comb: zeros(),
            unvoiced_weight: zeros(),
        }
    }

    pub fn reset(&mut self) {
        self.presence = 0.0;
        self.center.reset();
        self.pitch.reset();
        self.flux.reset();
        self.reset_gains();
    }

    pub fn reset_gains(&mut self) {
        self.mask.reset();
        self.reverb.reset();
    }

    pub fn mid_gains(&self) -> &[f32] {
        self.mask.gains()
    }

    pub fn side_gains(&self) -> &[f32] {
        self.reverb.gains()
    }

    pub fn analyze(&mut self, vocal_level: f32, left: &mut [Complex32], right: &mut [Complex32]) {
        if vocal_level >= 1.0 {
            return;
        }
        for k in 0..left.len() {
            let mid = left[k].add(right[k]).scale(0.5);
            let side = left[k].sub(right[k]).scale(0.5);
            self.mid_power[k] = mid.norm_sqr();
            self.mid_magnitude[k] = self.mid_power[k].sqrt();
            self.side_power[k] = side.norm_sqr();
        }
        self.center.update(left, right, &mut self.center_prob);
        for k in 0..left.len() {
            self.centered_magnitude[k] = self.mid_magnitude[k] * self.center_prob[k];
        }
        let pitch = self.pitch.update(&self.centered_magnitude);
        harmonic_comb(pitch.f0_hz, self.bin_hz, self.comb_max_hz, self.vibrato_cents, &mut self.comb);
        let flux = self.flux.update(&self.mid_magnitude);
        self.presence = if pitch.voicing > self.presence {
            pitch.voicing
        } else {
            self.presence + self.presence_release * (pitch.voicing - self.presence)
        };
        self.unvoiced.update(
            &self.mid_magnitude,
            flux.low,
            self.presence,
            pitch.voicing,
            &mut self.unvoiced_weight,
        );
        let depth = (1.0 - vocal_level).clamp(0.0, 1.0).powf(DEPTH_CURVE);
        self.mask.build(&MaskInputs {
            center: &self.center_prob,
            comb: &self.comb,
            unvoiced: &self.unvoiced_weight,
            voicing: pitch.voicing,
            mono_confidence: self.center.mono_confidence(),
            broad_flux: flux.broad,
            depth,
        });
        self.reverb.update(self.mask.gains(), &self.mid_power, &self.side_power, depth);
        apply_gains(self.mask.gains(), self.reverb.gains(), left, right);
    }
}

/// `L' = g*M + g_s*S`, `R' = g*M - g_s*S` per bin.
pub fn apply_gains(
    mid_gain: &[f32],
    side_gain: &[f32],
    left: &mut [Complex32],
    right: &mut [Complex32],
) {
    for k in 0..left.len() {
        let mid = left[k].add(right[k]).scale(0.5 * mid_gain[k]);
        let side = left[k].sub(right[k]).scale(0.5 * side_gain[k]);
        left[k] = mid.add(side);
        right[k] = mid.sub(side);
    }
}
