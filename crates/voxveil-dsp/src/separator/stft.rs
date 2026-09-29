use std::f32::consts::TAU;

use super::fft::{Complex32, Fft, pack_real_pair, split_real_pair};

/// Streaming STFT with sqrt-Hann analysis/synthesis windows and overlap-add.
/// Output is delayed by exactly `size - 1` frames. All buffers are allocated in `new`.
pub struct Stft {
    fft: Fft,
    size: usize,
    hop: usize,
    window: Box<[f32]>,
    input_l: Box<[f32]>,
    input_r: Box<[f32]>,
    input_len: usize,
    ola_l: Box<[f32]>,
    ola_r: Box<[f32]>,
    ola_norm: Box<[f32]>,
    queue: Box<[[f32; 2]]>,
    queue_pos: usize,
    queue_len: usize,
    packed: Box<[Complex32]>,
    spectrum_l: Box<[Complex32]>,
    spectrum_r: Box<[Complex32]>,
}

impl Stft {
    pub fn new(size: usize, hop: usize) -> Self {
        assert!(hop > 0 && hop <= size / 2 && size % hop == 0, "invalid STFT hop");
        let window = (0..size)
            .map(|i| (0.5 - 0.5 * (TAU * i as f32 / size as f32).cos()).max(0.0).sqrt())
            .collect();
        let bins = size / 2 + 1;
        let zeros = || vec![0.0_f32; size].into_boxed_slice();
        Self {
            fft: Fft::new(size),
            size,
            hop,
            window,
            input_l: zeros(),
            input_r: zeros(),
            input_len: 0,
            ola_l: zeros(),
            ola_r: zeros(),
            ola_norm: zeros(),
            queue: vec![[0.0; 2]; hop].into_boxed_slice(),
            queue_pos: 0,
            queue_len: 0,
            packed: vec![Complex32::ZERO; size].into_boxed_slice(),
            spectrum_l: vec![Complex32::ZERO; bins].into_boxed_slice(),
            spectrum_r: vec![Complex32::ZERO; bins].into_boxed_slice(),
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn hop(&self) -> usize {
        self.hop
    }

    #[cfg(test)]
    pub fn bins(&self) -> usize {
        self.size / 2 + 1
    }

    pub fn latency_frames(&self) -> usize {
        self.size - 1
    }

    pub fn reset(&mut self) {
        for buffer in [
            &mut self.input_l,
            &mut self.input_r,
            &mut self.ola_l,
            &mut self.ola_r,
            &mut self.ola_norm,
        ] {
            buffer.fill(0.0);
        }
        self.queue.fill([0.0; 2]);
        self.input_len = 0;
        self.queue_pos = 0;
        self.queue_len = 0;
    }

    pub fn push<F>(&mut self, left: f32, right: f32, analyze: &mut F) -> [f32; 2]
    where
        F: FnMut(&mut [Complex32], &mut [Complex32]),
    {
        self.input_l[self.input_len] = finite_or_zero(left);
        self.input_r[self.input_len] = finite_or_zero(right);
        self.input_len += 1;
        if self.input_len == self.size {
            self.run_hop(analyze);
            self.input_l.copy_within(self.hop.., 0);
            self.input_r.copy_within(self.hop.., 0);
            self.input_len = self.size - self.hop;
        }
        if self.queue_len == 0 {
            return [0.0; 2];
        }
        let output = self.queue[self.queue_pos];
        self.queue_pos += 1;
        self.queue_len -= 1;
        output
    }

    fn run_hop<F>(&mut self, analyze: &mut F)
    where
        F: FnMut(&mut [Complex32], &mut [Complex32]),
    {
        for i in 0..self.size {
            let w = self.window[i];
            self.packed[i] = Complex32::new(self.input_l[i] * w, self.input_r[i] * w);
        }
        self.fft.process(&mut self.packed, false);
        split_real_pair(&self.packed, &mut self.spectrum_l, &mut self.spectrum_r);
        analyze(&mut self.spectrum_l, &mut self.spectrum_r);
        pack_real_pair(&self.spectrum_l, &self.spectrum_r, &mut self.packed);
        self.fft.process(&mut self.packed, true);

        for i in 0..self.size {
            let w = self.window[i];
            self.ola_l[i] += self.packed[i].re * w;
            self.ola_r[i] += self.packed[i].im * w;
            self.ola_norm[i] += w * w;
        }
        debug_assert_eq!(self.queue_len, 0);
        for i in 0..self.hop {
            let norm = self.ola_norm[i];
            let (l, r) = if norm > 1.0e-6 {
                (self.ola_l[i] / norm, self.ola_r[i] / norm)
            } else {
                (0.0, 0.0)
            };
            self.queue[i] = [finite_or_zero(l), finite_or_zero(r)];
        }
        self.queue_pos = 0;
        self.queue_len = self.hop;

        let hop = self.hop;
        for buffer in [&mut self.ola_l, &mut self.ola_r, &mut self.ola_norm] {
            buffer.copy_within(hop.., 0);
            let tail = buffer.len() - hop;
            buffer[tail..].fill(0.0);
        }
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(frames: usize) -> Vec<[f32; 2]> {
        let mut state = 7_u32;
        let mut next = move || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) as f32 / (1_u32 << 24) as f32 - 0.5
        };
        (0..frames).map(|_| [next(), next()]).collect()
    }

    fn run(stft: &mut Stft, input: &[[f32; 2]]) -> Vec<[f32; 2]> {
        let mut identity = |_: &mut [Complex32], _: &mut [Complex32]| {};
        input.iter().map(|&[l, r]| stft.push(l, r, &mut identity)).collect()
    }

    #[test]
    fn identity_analysis_reconstructs_input_after_latency() {
        let mut stft = Stft::new(4_096, 1_024);
        assert_eq!(stft.latency_frames(), 4_095);
        assert_eq!(stft.bins(), 2_049);
        let input = noise(20_000);
        let output = run(&mut stft, &input);
        let latency = stft.latency_frames();
        for n in (latency + 4_096)..input.len() {
            let [l, r] = input[n - latency];
            assert!((output[n][0] - l).abs() < 1.0e-4, "left frame {n}");
            assert!((output[n][1] - r).abs() < 1.0e-4, "right frame {n}");
        }
    }

    #[test]
    fn impulse_appears_exactly_after_latency() {
        let mut stft = Stft::new(1_024, 256);
        let mut input = vec![[0.0; 2]; 6_000];
        input[3_000] = [1.0, -0.5];
        let output = run(&mut stft, &input);
        let peak = output
            .iter()
            .enumerate()
            .max_by(|a, b| a.1[0].abs().total_cmp(&b.1[0].abs()))
            .unwrap()
            .0;
        assert_eq!(peak, 3_000 + 1_023);
        assert!((output[peak][0] - 1.0).abs() < 1.0e-4);
        assert!((output[peak][1] + 0.5).abs() < 1.0e-4);
    }

    #[test]
    fn non_finite_input_produces_finite_output() {
        let mut stft = Stft::new(1_024, 256);
        let mut input = noise(5_000);
        input[1_000] = [f32::NAN, f32::INFINITY];
        input[2_000] = [f32::NEG_INFINITY, 0.0];
        assert!(run(&mut stft, &input).iter().all(|f| f[0].is_finite() && f[1].is_finite()));
    }

    #[test]
    fn reset_discards_history() {
        let mut stft = Stft::new(1_024, 256);
        run(&mut stft, &noise(4_000));
        stft.reset();
        let output = run(&mut stft, &vec![[0.0; 2]; 4_000]);
        assert!(output.iter().all(|f| f[0] == 0.0 && f[1] == 0.0));
    }
}
