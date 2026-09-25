use std::f32::consts::TAU;

use voxveil_audio_core::AudioProcessor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

#[path = "spectral_center_support.rs"]
mod support;

use support::{Complex32, ProfileParameters, fft, frequency_weight};

const FFT_SIZE: usize = 512;
const HOP_SIZE: usize = 128;
const LATENCY_FRAMES: usize = FFT_SIZE - 1;
const EPSILON: f32 = 1.0e-9;

pub struct SpectralCenterSuppressor {
    sample_rate: f32,
    vocal_level: VocalLevel,
    profile: ClassicSuppressionProfile,
    input_l: [f32; FFT_SIZE],
    input_r: [f32; FFT_SIZE],
    input_len: usize,
    ola_l: [f32; FFT_SIZE],
    ola_r: [f32; FFT_SIZE],
    ola_norm: [f32; FFT_SIZE],
    output_queue: [[f32; 2]; HOP_SIZE],
    output_queue_pos: usize,
    output_queue_len: usize,
    window: [f32; FFT_SIZE],
    bit_reversal: [usize; FFT_SIZE],
    twiddles: [Complex32; FFT_SIZE / 2],
    fft_l: [Complex32; FFT_SIZE],
    fft_r: [Complex32; FFT_SIZE],
    previous_mid_magnitude: [f32; FFT_SIZE],
    smoothed_gain: [f32; FFT_SIZE],
    smoothed_side_to_mid_ratio: f32,
}

impl SpectralCenterSuppressor {
    pub fn new(
        sample_rate: u32,
        vocal_level: VocalLevel,
        profile: ClassicSuppressionProfile,
    ) -> Self {
        let mut processor = Self {
            sample_rate: sample_rate.max(8_000) as f32,
            vocal_level,
            profile,
            input_l: [0.0; FFT_SIZE],
            input_r: [0.0; FFT_SIZE],
            input_len: 0,
            ola_l: [0.0; FFT_SIZE],
            ola_r: [0.0; FFT_SIZE],
            ola_norm: [0.0; FFT_SIZE],
            output_queue: [[0.0; 2]; HOP_SIZE],
            output_queue_pos: 0,
            output_queue_len: 0,
            window: [0.0; FFT_SIZE],
            bit_reversal: [0; FFT_SIZE],
            twiddles: [Complex32::default(); FFT_SIZE / 2],
            fft_l: [Complex32::default(); FFT_SIZE],
            fft_r: [Complex32::default(); FFT_SIZE],
            previous_mid_magnitude: [0.0; FFT_SIZE],
            smoothed_gain: [1.0; FFT_SIZE],
            smoothed_side_to_mid_ratio: 0.0,
        };
        processor.prepare_tables();
        processor
    }

    pub fn set_vocal_level(&mut self, vocal_level: VocalLevel) {
        if vocal_level.get() >= 1.0 {
            self.smoothed_gain.fill(1.0);
        }
        self.vocal_level = vocal_level;
    }

    pub fn set_profile(&mut self, profile: ClassicSuppressionProfile) {
        if self.profile != profile
            && (self.profile == ClassicSuppressionProfile::Strong
                || profile == ClassicSuppressionProfile::Strong)
        {
            self.smoothed_gain.fill(1.0);
        }
        if self.profile != ClassicSuppressionProfile::MusicPreservation
            && profile == ClassicSuppressionProfile::MusicPreservation
        {
            self.smoothed_gain.fill(1.0);
        }
        self.profile = profile;
    }

    fn prepare_tables(&mut self) {
        let bits = FFT_SIZE.trailing_zeros();
        for index in 0..FFT_SIZE {
            let reversed = index.reverse_bits() >> (usize::BITS - bits);
            self.bit_reversal[index] = reversed;
            let hann = 0.5 - 0.5 * (TAU * index as f32 / FFT_SIZE as f32).cos();
            self.window[index] = hann.max(0.0).sqrt();
        }
        for (index, twiddle) in self.twiddles.iter_mut().enumerate() {
            let phase = -TAU * index as f32 / FFT_SIZE as f32;
            let (sin, cos) = phase.sin_cos();
            *twiddle = Complex32::new(cos, sin);
        }
    }

    fn process_window(&mut self) {
        for index in 0..FFT_SIZE {
            self.fft_l[index] = Complex32::new(self.input_l[index] * self.window[index], 0.0);
            self.fft_r[index] = Complex32::new(self.input_r[index] * self.window[index], 0.0);
        }

        fft(&mut self.fft_l, &self.bit_reversal, &self.twiddles, false);
        fft(&mut self.fft_r, &self.bit_reversal, &self.twiddles, false);

        let parameters = ProfileParameters::for_profile(self.profile);
        let suppression = (1.0 - self.vocal_level.get()).clamp(0.0, 1.0).powf(0.72);

        if self.profile == ClassicSuppressionProfile::Strong {
            let (mid_power, side_power) = self.fft_l.iter().zip(&self.fft_r).fold(
                (0.0_f64, 0.0_f64),
                |(mid_total, side_total), (left, right)| {
                    let mid_re = f64::from((left.re + right.re) * 0.5);
                    let mid_im = f64::from((left.im + right.im) * 0.5);
                    let side_re = f64::from((left.re - right.re) * 0.5);
                    let side_im = f64::from((left.im - right.im) * 0.5);
                    (
                        mid_total + mid_re * mid_re + mid_im * mid_im,
                        side_total + side_re * side_re + side_im * side_im,
                    )
                },
            );
            let ratio = (side_power / (mid_power + f64::from(EPSILON))) as f32;
            let ratio = if ratio.is_finite() { ratio.clamp(0.0, 1.0) } else { 0.0 };
            let smoothing = 1.0 - (-((HOP_SIZE as f32) / (self.sample_rate * 0.020))).exp();
            self.smoothed_side_to_mid_ratio +=
                smoothing * (ratio - self.smoothed_side_to_mid_ratio);
        } else {
            self.smoothed_side_to_mid_ratio = 0.0;
        }
        let floor_gain = if self.profile == ClassicSuppressionProfile::Strong {
            let stereo_confidence =
                ((self.smoothed_side_to_mid_ratio - 0.001) / 0.009).clamp(0.0, 1.0);
            0.5 + stereo_confidence * (parameters.floor_gain - 0.5)
        } else {
            parameters.floor_gain
        };

        for index in 0..FFT_SIZE {
            let left = self.fft_l[index];
            let right = self.fft_r[index];
            let magnitude_l = left.magnitude();
            let magnitude_r = right.magnitude();
            let balance =
                1.0 - (magnitude_l - magnitude_r).abs() / (magnitude_l + magnitude_r + EPSILON);
            let cross_real = left.re * right.re + left.im * right.im;
            let coherence =
                (cross_real / (magnitude_l * magnitude_r + EPSILON)).clamp(0.0, 1.0);
            let center_likelihood = (balance * coherence).clamp(0.0, 1.0);

            let mid = left.add(right).scale(0.5);
            let side = left.sub(right).scale(0.5);
            let mid_magnitude = mid.magnitude();
            let previous = self.previous_mid_magnitude[index];
            let positive_flux = (mid_magnitude - previous).max(0.0) / (previous + 0.02);
            let transient = (positive_flux / (positive_flux + 0.5)).clamp(0.0, 1.0);
            self.previous_mid_magnitude[index] = mid_magnitude;

            let mirrored_bin = index.min(FFT_SIZE - index);
            let frequency_hz = mirrored_bin as f32 * self.sample_rate / FFT_SIZE as f32;
            let frequency_weight = frequency_weight(frequency_hz, parameters);
            let transient_weight = 1.0 - transient * parameters.transient_protection;
            let center_weight = center_likelihood * center_likelihood;
            let attenuation = (1.0 - parameters.floor_gain)
                * suppression
                * center_weight
                * frequency_weight
                * transient_weight;
            let attenuation = (1.0 - floor_gain) / (1.0 - parameters.floor_gain)
                * attenuation;
            let target_gain = (1.0 - attenuation).clamp(floor_gain, 1.0);
            let previous_gain = self.smoothed_gain[index];
            let smoothing = if target_gain < previous_gain { 0.55 } else { 0.14 };
            let gain = previous_gain + smoothing * (target_gain - previous_gain);
            self.smoothed_gain[index] = gain;

            let processed_mid = mid.scale(gain);
            self.fft_l[index] = processed_mid.add(side);
            self.fft_r[index] = processed_mid.sub(side);
        }

        fft(&mut self.fft_l, &self.bit_reversal, &self.twiddles, true);
        fft(&mut self.fft_r, &self.bit_reversal, &self.twiddles, true);

        for index in 0..FFT_SIZE {
            let window = self.window[index];
            self.ola_l[index] += self.fft_l[index].re * window;
            self.ola_r[index] += self.fft_r[index].re * window;
            self.ola_norm[index] += window * window;
        }

        debug_assert_eq!(self.output_queue_len, 0);
        self.output_queue_pos = 0;
        self.output_queue_len = HOP_SIZE;
        for index in 0..HOP_SIZE {
            let norm = self.ola_norm[index];
            let left = if norm > 1.0e-6 { self.ola_l[index] / norm } else { 0.0 };
            let right = if norm > 1.0e-6 { self.ola_r[index] / norm } else { 0.0 };
            self.output_queue[index] = [
                if left.is_finite() { left } else { 0.0 },
                if right.is_finite() { right } else { 0.0 },
            ];
        }

        self.ola_l.copy_within(HOP_SIZE..FFT_SIZE, 0);
        self.ola_r.copy_within(HOP_SIZE..FFT_SIZE, 0);
        self.ola_norm.copy_within(HOP_SIZE..FFT_SIZE, 0);
        self.ola_l[FFT_SIZE - HOP_SIZE..].fill(0.0);
        self.ola_r[FFT_SIZE - HOP_SIZE..].fill(0.0);
        self.ola_norm[FFT_SIZE - HOP_SIZE..].fill(0.0);
    }

    fn push_input(&mut self, left: f32, right: f32) {
        self.input_l[self.input_len] = left;
        self.input_r[self.input_len] = right;
        self.input_len += 1;
        if self.input_len == FFT_SIZE {
            self.process_window();
            self.input_l.copy_within(HOP_SIZE..FFT_SIZE, 0);
            self.input_r.copy_within(HOP_SIZE..FFT_SIZE, 0);
            self.input_len = FFT_SIZE - HOP_SIZE;
        }
    }

    fn pop_output(&mut self) -> [f32; 2] {
        if self.output_queue_len == 0 {
            return [0.0; 2];
        }
        let output = self.output_queue[self.output_queue_pos];
        self.output_queue_pos += 1;
        self.output_queue_len -= 1;
        output
    }
}

impl AudioProcessor for SpectralCenterSuppressor {
    fn process_stereo_interleaved(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_exact_mut(2) {
            self.push_input(frame[0], frame[1]);
            let [left, right] = self.pop_output();
            frame[0] = left;
            frame[1] = right;
        }
    }

    fn latency_frames(&self) -> usize {
        LATENCY_FRAMES
    }
}

#[cfg(test)]
#[path = "spectral_center_tests.rs"]
mod tests;
