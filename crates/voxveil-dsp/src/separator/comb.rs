/// Writes Gaussian harmonic weights for `f0_hz` into `comb` (bins spaced `bin_hz`).
/// Width per harmonic is `max(1.5 bins, h*f0*(2^(vibrato_cents/1200) - 1))`, so the
/// Hann main lobe and the vibrato excursion stay inside the comb.
pub fn harmonic_comb(f0_hz: f32, bin_hz: f32, max_hz: f32, vibrato_cents: f32, comb: &mut [f32]) {
    comb.fill(0.0);
    if !f0_hz.is_finite() || f0_hz <= 0.0 || comb.is_empty() {
        return;
    }
    let spread = (vibrato_cents / 1_200.0).exp2() - 1.0;
    let last = comb.len() - 1;
    let mut harmonic = 1_u32;
    loop {
        let centre = f0_hz * harmonic as f32;
        if centre > max_hz || centre / bin_hz > last as f32 {
            break;
        }
        let sigma = (1.5 * bin_hz).max(centre * spread);
        let lo = ((centre - 3.0 * sigma) / bin_hz).floor().max(0.0) as usize;
        let hi = (((centre + 3.0 * sigma) / bin_hz).ceil() as usize).min(last);
        let inverse = 1.0 / (2.0 * sigma * sigma);
        for (k, weight) in comb.iter_mut().enumerate().take(hi + 1).skip(lo) {
            let distance = k as f32 * bin_hz - centre;
            let value = (-distance * distance * inverse).exp();
            if value > *weight {
                *weight = value;
            }
        }
        harmonic += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIN_HZ: f32 = 48_000.0 / 4_096.0;

    fn bin(hz: f32) -> usize {
        (hz / BIN_HZ).round() as usize
    }

    #[test]
    fn weights_peak_on_harmonics_and_vanish_between_them() {
        let mut comb = vec![0.0; 2_049];
        harmonic_comb(220.0, BIN_HZ, 5_000.0, 50.0, &mut comb);
        assert!(comb[bin(220.0)] > 0.9);
        assert!(comb[bin(440.0)] > 0.9);
        assert!(comb[bin(880.0)] > 0.9);
        assert!(comb[bin(770.0)] < 0.1, "between 660 and 880: {}", comb[bin(770.0)]);
        assert!(comb[bin(1_210.0)] < 0.1, "between 1100 and 1320: {}", comb[bin(1_210.0)]);
    }

    #[test]
    fn nothing_above_max_frequency_and_nothing_when_unvoiced() {
        let mut comb = vec![0.0; 2_049];
        harmonic_comb(220.0, BIN_HZ, 5_000.0, 50.0, &mut comb);
        assert!(comb[bin(5_500.0)..].iter().all(|&w| w == 0.0));
        harmonic_comb(0.0, BIN_HZ, 5_000.0, 50.0, &mut comb);
        assert!(comb.iter().all(|&w| w == 0.0));
    }

    #[test]
    fn higher_harmonics_are_wider_to_cover_vibrato() {
        let mut comb = vec![0.0; 2_049];
        harmonic_comb(220.0, BIN_HZ, 5_000.0, 50.0, &mut comb);
        let low = comb[bin(220.0) + 2];
        let high = comb[bin(220.0 * 15.0) + 2];
        assert!(high > low, "low={low} high={high}");
    }
}
