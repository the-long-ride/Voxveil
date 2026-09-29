use std::f64::consts::TAU;

use super::fft::Complex32;
use super::params::SeparatorParams;
use super::pitch::PitchTracker;
use super::stft::Stft;

const RATE: f64 = 48_000.0;
const SKIP_HOPS: usize = 10;

fn track(samples: &[f32]) -> Vec<(f32, f32)> {
    let params = SeparatorParams::default();
    let mut stft = Stft::new(4_096, 1_024);
    let mut tracker =
        PitchTracker::new(2_049, (RATE / 4_096.0) as f32, (1_024.0 / RATE) as f32, &params);
    let mut magnitude = vec![0.0_f32; 2_049];
    let mut estimates = Vec::new();
    let mut analyze = |l: &mut [Complex32], r: &mut [Complex32]| {
        for k in 0..magnitude.len() {
            magnitude[k] = l[k].add(r[k]).scale(0.5).magnitude();
        }
        let estimate = tracker.update(&magnitude);
        estimates.push((estimate.f0_hz, estimate.voicing));
    };
    for &sample in samples {
        stft.push(sample, sample, &mut analyze);
    }
    estimates.split_off(SKIP_HOPS)
}

fn harmonic_tone(f0: f64, harmonics: usize, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|n| {
            let t = n as f64 / RATE;
            let value: f64 =
                (1..=harmonics).map(|h| (TAU * f0 * h as f64 * t).sin() / h as f64).sum();
            (value * 0.1) as f32
        })
        .collect()
}

fn vibrato_tone(frames: usize) -> Vec<f32> {
    let mut phase = 0.0_f64;
    (0..frames)
        .map(|n| {
            let t = n as f64 / RATE;
            let f = 220.0 * (50.0 / 1_200.0 * (TAU * 5.5 * t).sin()).exp2();
            phase += TAU * f / RATE;
            let value: f64 = (1..=10).map(|h| (phase * h as f64).sin() / h as f64).sum();
            (value * 0.1) as f32
        })
        .collect()
}

fn noise(frames: usize) -> Vec<f32> {
    let mut state = 3_u32;
    (0..frames)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((state >> 8) as f32 / (1_u32 << 24) as f32 - 0.5) * 0.2
        })
        .collect()
}

fn cents(estimate: f32, reference: f32) -> f32 {
    1_200.0 * (estimate / reference).log2()
}

fn mean_voicing(estimates: &[(f32, f32)]) -> f32 {
    estimates.iter().map(|e| e.1).sum::<f32>() / estimates.len() as f32
}

#[test]
fn steady_harmonic_tone_is_tracked() {
    for f0 in [110.0_f32, 220.0, 440.0] {
        let estimates = track(&harmonic_tone(f64::from(f0), 10, 96_000));
        for &(estimate, voicing) in &estimates {
            assert!(voicing > 0.6, "f0={f0} voicing={voicing}");
            assert!(cents(estimate, f0).abs() < 25.0, "f0={f0} estimate={estimate}");
        }
    }
}

#[test]
fn vibrato_voice_is_tracked_without_octave_errors() {
    let estimates = track(&vibrato_tone(96_000));
    for &(estimate, voicing) in &estimates {
        assert!(voicing > 0.5, "voicing={voicing}");
        assert!(cents(estimate, 220.0).abs() <= 75.0, "estimate={estimate}");
    }
}

#[test]
fn white_noise_is_not_voiced() {
    let voicing = mean_voicing(&track(&noise(96_000)));
    assert!(voicing < 0.2, "voicing={voicing}");
}

#[test]
fn low_bass_below_the_vocal_range_is_not_voiced() {
    let samples: Vec<f32> = (0..96_000)
        .map(|n| {
            let t = n as f64 / RATE;
            let value = (TAU * 50.0 * t).sin()
                + 0.5 * (TAU * 100.0 * t).sin()
                + 0.25 * (TAU * 150.0 * t).sin();
            (value * 0.2) as f32
        })
        .collect();
    let voicing = mean_voicing(&track(&samples));
    assert!(voicing < 0.3, "voicing={voicing}");
}

#[test]
fn silence_is_not_voiced() {
    let estimates = track(&vec![0.0; 48_000]);
    assert!(estimates.iter().all(|&(f0, voicing)| f0 == 0.0 && voicing == 0.0));
}
