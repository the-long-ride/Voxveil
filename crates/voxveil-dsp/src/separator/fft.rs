#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex32 {
    pub re: f32,
    pub im: f32,
}

impl Complex32 {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }

    pub fn sub(self, other: Self) -> Self {
        Self::new(self.re - other.re, self.im - other.im)
    }

    pub fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    pub fn scale(self, value: f32) -> Self {
        Self::new(self.re * value, self.im * value)
    }

    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }

    pub fn norm_sqr(self) -> f32 {
        self.re * self.re + self.im * self.im
    }

    pub fn magnitude(self) -> f32 {
        self.norm_sqr().sqrt()
    }

    /// Multiplies by `i`.
    pub fn mul_i(self) -> Self {
        Self::new(-self.im, self.re)
    }

    /// Divides by `i`.
    pub fn div_i(self) -> Self {
        Self::new(self.im, -self.re)
    }
}

/// In-place radix-2 FFT for a size fixed at construction. Tables are built once.
pub struct Fft {
    size: usize,
    bit_reversal: Box<[usize]>,
    twiddles: Box<[Complex32]>,
}

impl Fft {
    pub fn new(size: usize) -> Self {
        assert!(size >= 4 && size.is_power_of_two(), "FFT size must be a power of two >= 4");
        let bits = size.trailing_zeros();
        let bit_reversal = (0..size)
            .map(|index| index.reverse_bits() >> (usize::BITS - bits))
            .collect();
        let twiddles = (0..size / 2)
            .map(|index| {
                let phase = -std::f64::consts::TAU * index as f64 / size as f64;
                Complex32::new(phase.cos() as f32, phase.sin() as f32)
            })
            .collect();
        Self { size, bit_reversal, twiddles }
    }

    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn process(&self, buffer: &mut [Complex32], inverse: bool) {
        debug_assert_eq!(buffer.len(), self.size);
        for (index, &reversed) in self.bit_reversal.iter().enumerate() {
            if reversed > index {
                buffer.swap(index, reversed);
            }
        }
        let mut len = 2;
        while len <= self.size {
            let half = len / 2;
            let step = self.size / len;
            for start in (0..self.size).step_by(len) {
                for offset in 0..half {
                    let mut twiddle = self.twiddles[offset * step];
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
            let scale = 1.0 / self.size as f32;
            for value in buffer.iter_mut() {
                *value = value.scale(scale);
            }
        }
    }
}

/// Splits the spectrum of `left + i*right` into the half spectra (bins `0..=N/2`)
/// of the two real signals.
pub fn split_real_pair(z: &[Complex32], left: &mut [Complex32], right: &mut [Complex32]) {
    let n = z.len();
    for k in 0..=n / 2 {
        let a = z[k];
        let b = z[(n - k) % n].conj();
        left[k] = a.add(b).scale(0.5);
        right[k] = a.sub(b).scale(0.5).div_i();
    }
}

/// Inverse of `split_real_pair`: rebuilds the full spectrum of `left + i*right`
/// from half spectra using Hermitian symmetry.
pub fn pack_real_pair(left: &[Complex32], right: &[Complex32], z: &mut [Complex32]) {
    let n = z.len();
    let half = n / 2;
    for k in 0..=half {
        z[k] = left[k].add(right[k].mul_i());
    }
    for k in 1..half {
        z[n - k] = left[k].conj().add(right[k].conj().mul_i());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(n: usize, seed: u32) -> Vec<f32> {
        let mut state = seed;
        (0..n)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 8) as f32 / (1_u32 << 24) as f32 - 0.5
            })
            .collect()
    }

    fn naive_dft(input: &[Complex32]) -> Vec<Complex32> {
        let n = input.len();
        (0..n)
            .map(|k| {
                input.iter().enumerate().fold(Complex32::ZERO, |acc, (t, x)| {
                    let phase = -std::f64::consts::TAU * (k * t) as f64 / n as f64;
                    acc.add(x.mul(Complex32::new(phase.cos() as f32, phase.sin() as f32)))
                })
            })
            .collect()
    }

    fn max_error(a: &[Complex32], b: &[Complex32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x.sub(*y).magnitude()).fold(0.0, f32::max)
    }

    #[test]
    fn forward_matches_naive_dft() {
        let input: Vec<Complex32> = signal(64, 1)
            .into_iter()
            .zip(signal(64, 2))
            .map(|(re, im)| Complex32::new(re, im))
            .collect();
        let mut fast = input.clone();
        Fft::new(64).process(&mut fast, false);
        let error = max_error(&fast, &naive_dft(&input));
        assert!(error < 1.0e-3, "error={error}");
    }

    #[test]
    fn inverse_restores_input_at_4096() {
        let input: Vec<Complex32> =
            signal(4_096, 3).into_iter().map(|re| Complex32::new(re, 0.0)).collect();
        let fft = Fft::new(4_096);
        let mut buffer = input.clone();
        fft.process(&mut buffer, false);
        fft.process(&mut buffer, true);
        let error = max_error(&buffer, &input);
        assert!(error < 1.0e-5, "error={error}");
    }

    #[test]
    fn packed_pair_matches_separate_transforms_and_round_trips() {
        let n = 1_024;
        let (left, right) = (signal(n, 4), signal(n, 5));
        let fft = Fft::new(n);
        let mut packed: Vec<Complex32> =
            left.iter().zip(&right).map(|(&l, &r)| Complex32::new(l, r)).collect();
        fft.process(&mut packed, false);
        let mut split_l = vec![Complex32::ZERO; n / 2 + 1];
        let mut split_r = vec![Complex32::ZERO; n / 2 + 1];
        split_real_pair(&packed, &mut split_l, &mut split_r);

        let mut only_l: Vec<Complex32> = left.iter().map(|&v| Complex32::new(v, 0.0)).collect();
        let mut only_r: Vec<Complex32> = right.iter().map(|&v| Complex32::new(v, 0.0)).collect();
        fft.process(&mut only_l, false);
        fft.process(&mut only_r, false);
        assert!(max_error(&split_l, &only_l[..=n / 2]) < 1.0e-3);
        assert!(max_error(&split_r, &only_r[..=n / 2]) < 1.0e-3);

        let mut repacked = vec![Complex32::ZERO; n];
        pack_real_pair(&split_l, &split_r, &mut repacked);
        assert!(max_error(&repacked, &packed) < 1.0e-3);
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn rejects_non_power_of_two_sizes() {
        let _ = Fft::new(1_000);
    }
}
