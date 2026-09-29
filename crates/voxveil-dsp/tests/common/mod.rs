#![allow(dead_code)]

use std::f64::consts::TAU;

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::{SeparatorEvalTap, VocalSeparator};
use voxveil_types::VocalLevel;

pub const RATE: u32 = 48_000;
pub const SETTLE_FRAMES: usize = 48_000;

/// Interleaved stereo buffer with the same value in both channels.
pub fn centered(mono: &[f32]) -> Vec<f32> {
    mono.iter().flat_map(|&v| [v, v]).collect()
}

/// Synthetic sung voice: 220 Hz, 5.5 Hz +/-50 cent vibrato, 10 harmonics at 1/h.
pub fn voice(frames: usize) -> Vec<f32> {
    let mut phase = 0.0_f64;
    (0..frames)
        .map(|n| {
            let t = n as f64 / f64::from(RATE);
            let f = 220.0 * (50.0 / 1_200.0 * (TAU * 5.5 * t).sin()).exp2();
            phase += TAU * f / f64::from(RATE);
            let value: f64 = (1..=10).map(|h| (phase * h as f64).sin() / h as f64).sum();
            (value * 0.1) as f32
        })
        .collect()
}

/// Noise bursts every 0.5 s with a 30 ms exponential decay (snare-like, broadband).
pub fn drums(frames: usize) -> Vec<f32> {
    let mut state = 9_u32;
    (0..frames)
        .map(|n| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (state >> 8) as f32 / (1_u32 << 24) as f32 - 0.5;
            let since_hit = (n % (RATE as usize / 2)) as f32 / RATE as f32;
            noise * 0.4 * (-since_hit / 0.03).exp()
        })
        .collect()
}

pub fn sine(frames: usize, hz: f64, amplitude: f32) -> Vec<f32> {
    (0..frames)
        .map(|n| (TAU * hz * n as f64 / f64::from(RATE)).sin() as f32 * amplitude)
        .collect()
}

/// Pure side content: left = +x, right = -x.
pub fn side(mono: &[f32]) -> Vec<f32> {
    mono.iter().flat_map(|&v| [v, -v]).collect()
}

pub fn add(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}

pub fn sub(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(x, y)| x - y).collect()
}

fn frame(buffer: &[f32], n: usize) -> [f32; 2] {
    if 2 * n + 1 < buffer.len() { [buffer[2 * n], buffer[2 * n + 1]] } else { [0.0; 2] }
}

/// Attenuation in dB of `component` inside the mix `component + rest` at Vocal 0,
/// measured with the separator's exact gains (after SETTLE_FRAMES).
pub fn attenuation_db(component: &[f32], rest: &[f32]) -> f64 {
    let mut separator = VocalSeparator::new(RATE, VocalLevel::new(0.0).unwrap());
    let mut tap = SeparatorEvalTap::new(&separator);
    let latency = separator.latency_frames();
    let frames = component.len() / 2;
    let (mut input, mut output) = (0.0_f64, 0.0_f64);
    for n in 0..frames + latency {
        let [_, out, _] = tap.process_frame(&mut separator, frame(component, n), frame(rest, n));
        if n >= latency + SETTLE_FRAMES && n - latency < frames {
            let [l, r] = frame(component, n - latency);
            input += f64::from(l * l + r * r);
            output += f64::from(out[0] * out[0] + out[1] * out[1]);
        }
    }
    10.0 * (input / output.max(1.0e-30)).log10()
}
