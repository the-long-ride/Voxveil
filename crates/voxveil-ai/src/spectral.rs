use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::sync::Arc;

pub(crate) struct Spectral {
    pub n: usize,
    pub frames: usize,
    pub bins: usize,
    centered: bool,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
}

impl Spectral {
    pub fn new(n: usize, frames: usize, bins: usize, centered: bool) -> Self {
        let mut planner = FftPlanner::new();
        Self {
            n,
            frames,
            bins,
            centered,
            forward: planner.plan_fft_forward(n),
            inverse: planner.plan_fft_inverse(n),
            window: (0..n)
                .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos())
                .collect(),
        }
    }

    fn offset(&self) -> usize {
        if self.centered { self.n / 2 } else { 0 }
    }

    pub fn analyze(&self, input: &[f32]) -> Vec<Complex32> {
        let length = input.len() / 2;
        let mut result = vec![Complex32::default(); 2 * self.frames * (self.n / 2 + 1)];
        let mut buffer = vec![Complex32::default(); self.n];
        for channel in 0..2 {
            for frame in 0..self.frames {
                for (i, value) in buffer.iter_mut().enumerate() {
                    let index = frame as isize * 1024 + i as isize - self.offset() as isize;
                    let reflected = if index < 0 {
                        -index
                    } else if index >= length as isize {
                        2 * length as isize - 2 - index
                    } else {
                        index
                    };
                    *value = Complex32::new(
                        input[reflected as usize * 2 + channel] * self.window[i],
                        0.0,
                    );
                }
                self.forward.process(&mut buffer);
                let start = (channel * self.frames + frame) * (self.n / 2 + 1);
                result[start..start + self.n / 2 + 1].copy_from_slice(&buffer[..self.n / 2 + 1]);
            }
        }
        result
    }

    pub fn synthesize(&self, spectra: &[Complex32], length: usize) -> Vec<f32> {
        let padded_length = (self.frames - 1) * 1024 + self.n;
        let mut output = vec![0.0; padded_length * 2];
        let mut norm = vec![0.0; padded_length];
        let mut buffer = vec![Complex32::default(); self.n];
        for channel in 0..2 {
            for frame in 0..self.frames {
                let start = (channel * self.frames + frame) * (self.n / 2 + 1);
                buffer[..self.n / 2 + 1].copy_from_slice(&spectra[start..start + self.n / 2 + 1]);
                for bin in 1..self.n / 2 {
                    buffer[self.n - bin] = buffer[bin].conj();
                }
                self.inverse.process(&mut buffer);
                for i in 0..self.n {
                    let position = frame * 1024 + i;
                    output[position * 2 + channel] += buffer[i].re * self.window[i] / self.n as f32;
                    if channel == 0 {
                        norm[position] += self.window[i] * self.window[i];
                    }
                }
            }
        }
        (self.offset()..self.offset() + length)
            .flat_map(|i| {
                let gain = if norm[i] > 1e-6 { 1.0 / norm[i] } else { 0.0 };
                [output[i * 2] * gain, output[i * 2 + 1] * gain]
            })
            .collect()
    }
}
