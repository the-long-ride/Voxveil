use voxveil_types::ClassicSuppressionProfile;

use super::FFT_SIZE;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Complex32 {
    pub(super) re: f32,
    pub(super) im: f32,
}

impl Complex32 {
    pub(super) const fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    pub(super) fn magnitude(self) -> f32 {
        (self.re * self.re + self.im * self.im).sqrt()
    }

    pub(super) fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }

    pub(super) fn sub(self, other: Self) -> Self {
        Self::new(self.re - other.re, self.im - other.im)
    }

    pub(super) fn scale(self, value: f32) -> Self {
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
pub(super) struct ProfileParameters {
    pub(super) floor_gain: f32,
    pub(super) low_protect_hz: f32,
    pub(super) low_full_hz: f32,
    pub(super) high_full_hz: f32,
    pub(super) high_protect_hz: f32,
    pub(super) transient_protection: f32,
}

impl ProfileParameters {
    pub(super) fn for_profile(profile: ClassicSuppressionProfile) -> Self {
        match profile {
            ClassicSuppressionProfile::MusicPreservation => Self {
                floor_gain: 0.251_188_64,
                low_protect_hz: 180.0,
                low_full_hz: 350.0,
                high_full_hz: 5_500.0,
                high_protect_hz: 9_000.0,
                transient_protection: 0.75,
            },
            ClassicSuppressionProfile::Balanced => Self {
                floor_gain: 0.125_892_55,
                low_protect_hz: 120.0,
                low_full_hz: 260.0,
                high_full_hz: 7_000.0,
                high_protect_hz: 12_000.0,
                transient_protection: 0.45,
            },
        }
    }
}

pub(super) fn fft(
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

pub(super) fn frequency_weight(frequency_hz: f32, parameters: ProfileParameters) -> f32 {
    let low = if frequency_hz <= parameters.low_protect_hz {
        0.0
    } else if frequency_hz >= parameters.low_full_hz {
        1.0
    } else {
        smoothstep(
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
            - smoothstep(
                (frequency_hz - parameters.high_full_hz)
                    / (parameters.high_protect_hz - parameters.high_full_hz),
            )
    };

    low * high
}
