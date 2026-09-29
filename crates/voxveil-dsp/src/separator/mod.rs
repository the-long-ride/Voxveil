mod center;
mod comb;
mod eval;
mod fft;
mod flux;
mod mask;
mod math;
mod params;
mod pitch;
mod reverb;
mod stages;
mod stft;
mod unvoiced;

#[cfg(test)]
mod tests_pitch;

use voxveil_audio_core::AudioProcessor;
use voxveil_types::VocalLevel;

pub use eval::SeparatorEvalTap;
pub use params::SeparatorParams;

use fft::Complex32;
use stages::Stages;
use stft::Stft;

/// FFT size giving ~11 Hz bins and ~85-93 ms latency at common sample rates.
pub fn separator_fft_size(sample_rate: u32) -> usize {
    if sample_rate > 50_000 { 8_192 } else { 4_096 }
}

/// Non-AI lead-vocal separator: center extraction + pitch comb + unvoiced detector +
/// side-reverb suppression on a 4096/1024 STFT (see the vocal separator design spec).
pub struct VocalSeparator {
    stft: Stft,
    stages: Stages,
    vocal_level: f32,
}

impl VocalSeparator {
    pub fn new(sample_rate: u32, vocal_level: VocalLevel) -> Self {
        Self::with_params(sample_rate, vocal_level, SeparatorParams::default())
    }

    pub fn with_params(sample_rate: u32, vocal_level: VocalLevel, params: SeparatorParams) -> Self {
        let rate = sample_rate.max(8_000);
        let size = separator_fft_size(rate);
        let hop = size / 4;
        let bin_hz = rate as f32 / size as f32;
        let hop_seconds = hop as f32 / rate as f32;
        Self {
            stft: Stft::new(size, hop),
            stages: Stages::new(size / 2 + 1, bin_hz, hop_seconds, params),
            vocal_level: vocal_level.get(),
        }
    }

    pub fn set_vocal_level(&mut self, vocal_level: VocalLevel) {
        if vocal_level.get() >= 1.0 {
            self.stages.reset_gains();
        }
        self.vocal_level = vocal_level.get();
    }

    /// Clears all signal history without reallocating.
    pub fn reset(&mut self) {
        self.stft.reset();
        self.stages.reset();
    }

    fn process_frame(&mut self, left: f32, right: f32) -> [f32; 2] {
        let level = self.vocal_level;
        let stages = &mut self.stages;
        self.stft.push(left, right, &mut |l: &mut [Complex32], r: &mut [Complex32]| {
            stages.analyze(level, l, r)
        })
    }
}

impl AudioProcessor for VocalSeparator {
    fn process_stereo_interleaved(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_exact_mut(2) {
            let [left, right] = self.process_frame(frame[0], frame[1]);
            frame[0] = left;
            frame[1] = right;
        }
    }

    fn latency_frames(&self) -> usize {
        self.stft.latency_frames()
    }
}
