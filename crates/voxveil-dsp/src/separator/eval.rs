use super::VocalSeparator;
use super::fft::Complex32;
use super::stages::apply_gains;
use super::stft::Stft;

/// Applies the separator's exact per-hop gains to the stems that were summed into the mix.
/// The mask is linear once computed, so `vocal_out + instrumental_out == mix_out` within
/// float error, which gives ground-truth per-stem attenuation. Test/evaluation use only.
pub struct SeparatorEvalTap {
    vocal: Stft,
    instrumental: Stft,
}

impl SeparatorEvalTap {
    pub fn new(separator: &VocalSeparator) -> Self {
        let (size, hop) = (separator.stft.size(), separator.stft.hop());
        Self { vocal: Stft::new(size, hop), instrumental: Stft::new(size, hop) }
    }

    /// Returns `[mix_out, vocal_out, instrumental_out]` for one frame.
    pub fn process_frame(
        &mut self,
        separator: &mut VocalSeparator,
        vocal: [f32; 2],
        instrumental: [f32; 2],
    ) -> [[f32; 2]; 3] {
        let mix_out =
            separator.process_frame(vocal[0] + instrumental[0], vocal[1] + instrumental[1]);
        let mid = separator.stages.mid_gains();
        let side = separator.stages.side_gains();
        let mut apply = |l: &mut [Complex32], r: &mut [Complex32]| apply_gains(mid, side, l, r);
        let vocal_out = self.vocal.push(vocal[0], vocal[1], &mut apply);
        let instrumental_out = self.instrumental.push(instrumental[0], instrumental[1], &mut apply);
        [mix_out, vocal_out, instrumental_out]
    }
}
