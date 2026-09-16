use std::f32::consts::TAU;

use voxveil_audio_core::AudioProcessor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

const FFT_SIZE: usize = 512;
const HOP_SIZE: usize = 128;
const LATENCY_FRAMES: usize = FFT_SIZE - 1;
const EPSILON: f32 = 1.0e-9;

#[derive(Clone, Copy, Debug, Default)]
struct Complex32 {
    re: f32,
    im: f32,
}

impl Complex32 {
    const fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    fn magnitude(self) -> f32 {
        (self.re * self.re + self.im * self.im).sqrt()
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.re - other.re, self.im - other.im)
    }

    fn scale(self, value: f32) -> Self {
        Self::new(self.re * value, self.im * value)
    }

    fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }
}

#[derive(Clone, Copy)]
struct ProfileParameters {
    floor_gain: f32,
    low_protect_hz: f32,
    low_full_hz: f32,
    high_full_hz: f32,
    high_protect_hz: f32,
    transient_protection: f32,
}

impl ProfileParameters {
    fn for_profile(profile: ClassicSuppressionProfile) -> Self {
        match profile {
            ClassicSuppressionProfile::MusicPreservation => Self {
                floor_gain: 0.251_188_64, // -12 dB
                low_protect_hz: 180.0,
                low_full_hz: 350.0,
                high_full_hz: 5_500.0,
                high_protect_hz: 9_000.0,
                transient_protection: 0.75,
            },
            ClassicSuppressionProfile::Balanced => Self {
                floor_gain: 0.125_892_55, // -18 dB
                low_protect_hz: 120.0,
                low_full_hz: 260.0,
                high_full_hz: 7_000.0,
                high_protect_hz: 12_000.0,
                transient_protection: 0.45,
            },
        }
    }
}

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

    fn fft(
        buffer: &mut [Complex32; FFT_SIZE],
        bit_reversal: &[usize; FFT_SIZE],
        twiddles: &[Complex32; FFT_SIZE / 2],
        inverse: bool,
    ) {
        for (index, &reversed) in bit_reversal.iter().enumerate() {
            if reversed > index {
                buffer.swap(index, reversed);
            }
        }

        let mut len = 2;
        while len <= FFT_SIZE {
            let half = len / 2;
            let twiddle_step = FFT_SIZE / len;
            for start in (0..FFT_SIZE).step_by(len) {
                for offset in 0..half {
                    let mut twiddle = twiddles[offset * twiddle_step];
                    if inverse {
                        twiddle.im = -twiddle.im;
                    }
                    let even = buffer[start + offset];
                    let odd = buffer[start + offset + half].mul(twiddle);
                    buffer[start + offset] = even.add(odd);
                    buffer[start + offset + half] = even.sub(odd);
                }
            }
            len *= 2;
        }

        if inverse {
            let scale = 1.0 / FFT_SIZE as f32;
            for value in buffer.iter_mut() {
                *value = value.scale(scale);
            }
        }
    }

    fn smoothstep(value: f32) -> f32 {
        let value = value.clamp(0.0, 1.0);
        value * value * (3.0 - 2.0 * value)
    }

    fn frequency_weight(frequency_hz: f32, parameters: ProfileParameters) -> f32 {
        let low = if frequency_hz <= parameters.low_protect_hz {
            0.0
        } else if frequency_hz >= parameters.low_full_hz {
            1.0
        } else {
            Self::smoothstep(
                (frequency_hz - parameters.low_protect_hz)
                    / (parameters.low_full_hz - parameters.low_protect_hz),
            )
        };

        let high = if frequency_hz <= parameters.high_full_hz {
            1.0
        } else if frequency_hz >= parameters.high_protect_hz {
            0.0
        } else {
            1.0
                - Self::smoothstep(
                    (frequency_hz - parameters.high_full_hz)
                        / (parameters.high_protect_hz - parameters.high_full_hz),
                )
        };

        low * high
    }

    fn process_window(&mut self) {
        for index in 0..FFT_SIZE {
            self.fft_l[index] = Complex32::new(self.input_l[index] * self.window[index], 0.0);
            self.fft_r[index] = Complex32::new(self.input_r[index] * self.window[index], 0.0);
        }

        Self::fft(
            &mut self.fft_l,
            &self.bit_reversal,
            &self.twiddles,
            false,
        );
        Self::fft(
            &mut self.fft_r,
            &self.bit_reversal,
            &self.twiddles,
            false,
        );

        let parameters = ProfileParameters::for_profile(self.profile);
        let suppression = (1.0 - self.vocal_level.get()).clamp(0.0, 1.0).powf(0.72);

        for index in 0..FFT_SIZE {
            let left = self.fft_l[index];
            let right = self.fft_r[index];
            let magnitude_l = left.magnitude();
            let magnitude_r = right.magnitude();
            let balance = 1.0
                - (magnitude_l - magnitude_r).abs()
                    / (magnitude_l + magnitude_r + EPSILON);
            let cross_real = left.re * right.re + left.im * right.im;
            let coherence = (cross_real / (magnitude_l * magnitude_r + EPSILON)).clamp(0.0, 1.0);
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
            let frequency_weight = Self::frequency_weight(frequency_hz, parameters);
            let transient_weight = 1.0 - transient * parameters.transient_protection;
            let center_weight = center_likelihood * center_likelihood;
            let attenuation = (1.0 - parameters.floor_gain)
                * suppression
                * center_weight
                * frequency_weight
                * transient_weight;
            let target_gain = (1.0 - attenuation).clamp(parameters.floor_gain, 1.0);
            let previous_gain = self.smoothed_gain[index];
            let smoothing = if target_gain < previous_gain { 0.55 } else { 0.14 };
            let gain = previous_gain + smoothing * (target_gain - previous_gain);
            self.smoothed_gain[index] = gain;

            let processed_mid = mid.scale(gain);
            self.fft_l[index] = processed_mid.add(side);
            self.fft_r[index] = processed_mid.sub(side);
        }

        Self::fft(
            &mut self.fft_l,
            &self.bit_reversal,
            &self.twiddles,
            true,
        );
        Self::fft(
            &mut self.fft_r,
            &self.bit_reversal,
            &self.twiddles,
            true,
        );

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
            let left = if norm > 1.0e-6 {
                self.ola_l[index] / norm
            } else {
                0.0
            };
            let right = if norm > 1.0e-6 {
                self.ola_r[index] / norm
            } else {
                0.0
            };
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
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 48_000;
    const FRAMES: usize = 8_192;
    const SETTLE_FRAMES: usize = 1_024;

    fn tone(freq_hz: f32, side_only: bool) -> Vec<f32> {
        let mut samples = Vec::with_capacity(FRAMES * 2);
        for frame in 0..FRAMES {
            let value = (TAU * freq_hz * frame as f32 / SAMPLE_RATE as f32).sin() * 0.4;
            samples.push(value);
            samples.push(if side_only { -value } else { value });
        }
        samples
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
        (sum / samples.len().max(1) as f32).sqrt()
    }

    fn render(
        input: &[f32],
        vocal_level: f32,
        profile: ClassicSuppressionProfile,
    ) -> (Vec<f32>, usize) {
        let mut processor = SpectralCenterSuppressor::new(
            SAMPLE_RATE,
            VocalLevel::new(vocal_level).unwrap(),
            profile,
        );
        let latency = processor.latency_frames();
        let mut stream = input.to_vec();
        stream.extend(std::iter::repeat_n(0.0, (latency + 1_024) * 2));
        for chunk in stream.chunks_mut(512) {
            processor.process_stereo_interleaved(chunk);
        }
        (stream, latency)
    }

    fn settled_left_channel(stream: &[f32], latency: usize) -> Vec<f32> {
        let start = (latency + SETTLE_FRAMES) * 2;
        let end = (latency + FRAMES - SETTLE_FRAMES) * 2;
        stream[start..end]
            .chunks_exact(2)
            .map(|frame| frame[0])
            .collect()
    }

    #[test]
    fn balanced_reduces_sustained_center_vocal_band_by_at_least_eight_db() {
        let input = tone(1_000.0, false);
        let input_rms = rms(&input[SETTLE_FRAMES * 2..(FRAMES - SETTLE_FRAMES) * 2]);
        let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
        let output_rms = rms(&settled_left_channel(&output, latency));
        assert!(
            output_rms / input_rms <= 0.40,
            "ratio={}",
            output_rms / input_rms
        );
    }

    #[test]
    fn music_preservation_keeps_centered_low_bass_within_three_db() {
        let input = tone(90.0, false);
        let input_rms = rms(&input[SETTLE_FRAMES * 2..(FRAMES - SETTLE_FRAMES) * 2]);
        let (output, latency) = render(
            &input,
            0.0,
            ClassicSuppressionProfile::MusicPreservation,
        );
        let output_rms = rms(&settled_left_channel(&output, latency));
        assert!(
            output_rms / input_rms >= 0.70,
            "ratio={}",
            output_rms / input_rms
        );
    }

    #[test]
    fn balanced_is_stronger_than_music_preservation_in_vocal_band() {
        let input = tone(1_000.0, false);
        let (music, music_latency) = render(
            &input,
            0.0,
            ClassicSuppressionProfile::MusicPreservation,
        );
        let (balanced, balanced_latency) =
            render(&input, 0.0, ClassicSuppressionProfile::Balanced);
        let music_rms = rms(&settled_left_channel(&music, music_latency));
        let balanced_rms = rms(&settled_left_channel(&balanced, balanced_latency));
        assert!(
            balanced_rms < music_rms * 0.8,
            "balanced={balanced_rms}, music={music_rms}"
        );
    }

    #[test]
    fn side_only_signal_is_preserved() {
        let input = tone(1_000.0, true);
        let input_rms = rms(&input[SETTLE_FRAMES * 2..(FRAMES - SETTLE_FRAMES) * 2]);
        let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
        let output_rms = rms(&settled_left_channel(&output, latency));
        let ratio = output_rms / input_rms;
        assert!((0.944..=1.059).contains(&ratio), "ratio={ratio}");
    }

    #[test]
    fn full_vocal_level_is_transparent_after_latency() {
        let input = tone(777.0, false);
        let input_left: Vec<f32> = input
            .chunks_exact(2)
            .map(|frame| frame[0])
            .collect();
        let (output, latency) = render(&input, 1.0, ClassicSuppressionProfile::Balanced);
        let output_left = settled_left_channel(&output, latency);
        let reference = &input_left[SETTLE_FRAMES..FRAMES - SETTLE_FRAMES];
        let max_error = output_left
            .iter()
            .zip(reference)
            .map(|(actual, expected)| (actual - expected).abs())
            .fold(0.0_f32, f32::max);
        assert!(max_error <= 0.002, "max_error={max_error}");
    }

    #[test]
    fn mono_signal_remains_audible_at_maximum_suppression() {
        let input = tone(1_000.0, false);
        for profile in [
            ClassicSuppressionProfile::MusicPreservation,
            ClassicSuppressionProfile::Balanced,
        ] {
            let (output, latency) = render(&input, 0.0, profile);
            assert!(rms(&settled_left_channel(&output, latency)) > 0.03);
        }
    }

    #[test]
    fn output_is_finite_and_latency_stays_below_twenty_five_ms() {
        let input = tone(1_000.0, false);
        let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(latency <= (SAMPLE_RATE as usize * 25 / 1_000));
    }
}
