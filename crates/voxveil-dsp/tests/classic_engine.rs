use std::f32::consts::TAU;

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::{ClassicEngine, SpectralCenterSuppressor};
use voxveil_types::{ClassicSuppressionProfile as Profile, VocalLevel};

const RATE: u32 = 48_000;

fn level(value: f32) -> VocalLevel {
    VocalLevel::new(value).unwrap()
}

fn mix(start: usize, frames: usize) -> Vec<f32> {
    (start..start + frames)
        .flat_map(|n| {
            let t = n as f32 / RATE as f32;
            let center = (TAU * 440.0 * t).sin() * 0.3;
            let side = (TAU * 1_234.0 * t).sin() * 0.1;
            [center + side, center - side]
        })
        .collect()
}

fn run(engine: &mut ClassicEngine, start: usize, frames: usize) -> Vec<f32> {
    let mut buffer = mix(start, frames);
    for chunk in buffer.chunks_mut(960) {
        engine.process_stereo_interleaved(chunk);
    }
    buffer
}

fn max_step(samples: &[f32]) -> f32 {
    samples
        .chunks_exact(2)
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]).abs().max((w[1][1] - w[0][1]).abs()))
        .fold(0.0, f32::max)
}

#[test]
fn legacy_profiles_behave_exactly_like_the_standalone_processor() {
    let mut engine = ClassicEngine::new(RATE, level(0.0), Profile::Balanced);
    let mut reference = SpectralCenterSuppressor::new(RATE, level(0.0), Profile::Balanced);
    let (mut a, mut b) = (mix(0, 9_600), mix(0, 9_600));
    engine.process_stereo_interleaved(&mut a);
    reference.process_stereo_interleaved(&mut b);
    engine.set_profile(Profile::Strong);
    reference.set_profile(Profile::Strong);
    let (mut c, mut d) = (mix(9_600, 9_600), mix(9_600, 9_600));
    engine.process_stereo_interleaved(&mut c);
    reference.process_stereo_interleaved(&mut d);
    assert_eq!(a, b);
    assert_eq!(c, d);
    assert_eq!(engine.latency_frames(), reference.latency_frames());
}

#[test]
fn switching_to_isolation_crossfades_without_steps() {
    let mut engine = ClassicEngine::new(RATE, level(0.0), Profile::Balanced);
    let mut output = run(&mut engine, 0, 48_000);
    engine.set_profile(Profile::Isolation);
    assert_eq!(engine.profile(), Profile::Isolation);
    assert_eq!(engine.latency_frames(), 511, "legacy stays audible while the separator warms");
    output.extend(run(&mut engine, 48_000, 48_000));
    assert_eq!(engine.latency_frames(), 4_095);
    assert!(output.iter().all(|v| v.is_finite()));
    let step = max_step(&output[1_024 * 2..]);
    assert!(step < 0.1, "max_step={step}");
}

#[test]
fn switching_back_to_legacy_restores_legacy_latency() {
    let mut engine = ClassicEngine::new(RATE, level(0.0), Profile::Isolation);
    assert_eq!(engine.latency_frames(), 4_095);
    let mut output = run(&mut engine, 0, 48_000);
    engine.set_profile(Profile::MusicPreservation);
    output.extend(run(&mut engine, 48_000, 48_000));
    assert_eq!(engine.latency_frames(), 511);
    assert!(output.iter().all(|v| v.is_finite()));
    let step = max_step(&output[8_192 * 2..]);
    assert!(step < 0.1, "max_step={step}");
}

#[test]
fn cancelling_a_switch_keeps_the_audible_engine() {
    let mut engine = ClassicEngine::new(RATE, level(0.0), Profile::Balanced);
    run(&mut engine, 0, 9_600);
    engine.set_profile(Profile::Isolation);
    run(&mut engine, 9_600, 960);
    engine.set_profile(Profile::Strong);
    let output = run(&mut engine, 10_560, 48_000);
    assert_eq!(engine.latency_frames(), 511);
    assert_eq!(engine.profile(), Profile::Strong);
    assert!(output.iter().all(|v| v.is_finite()));
}

#[test]
fn vocal_level_reaches_the_separator() {
    let mut engine = ClassicEngine::new(RATE, level(0.0), Profile::Isolation);
    run(&mut engine, 0, 24_000);
    engine.set_vocal_level(level(1.0));
    let latency = engine.latency_frames();
    let reference = mix(24_000, 24_000);
    let mut buffer = reference.clone();
    buffer.extend(std::iter::repeat_n(0.0, latency * 2));
    engine.process_stereo_interleaved(&mut buffer);
    let error = (4_096..24_000)
        .flat_map(|n| {
            let o = 2 * (n + latency);
            [(buffer[o] - reference[2 * n]).abs(), (buffer[o + 1] - reference[2 * n + 1]).abs()]
        })
        .fold(0.0, f32::max);
    assert!(error <= 0.002, "error={error}");
}
