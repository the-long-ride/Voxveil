use std::f32::consts::TAU;

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::SpectralCenterSuppressor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

const SAMPLE_RATE: u32 = 44_100;
const FRAMES: usize = 8_192;
const SETTLE_FRAMES: usize = 1_024;

fn tone(freq_hz: f32, side_only: bool) -> Vec<f32> {
    let mut samples = Vec::with_capacity(FRAMES * 2);
    for frame in 0..FRAMES {
        let value = (TAU * freq_hz * frame as f32 / SAMPLE_RATE as f32).sin() * 0.4;
        samples.push(value);
        samples.push(if side_only { -value } else { value });
    }
    samples
}

fn rms(samples: &[f32]) -> f32 {
    let sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (sum / samples.len().max(1) as f32).sqrt()
}

fn render(
    input: &[f32],
    vocal_level: f32,
    profile: ClassicSuppressionProfile,
) -> (Vec<f32>, usize) {
    let mut processor = SpectralCenterSuppressor::new(
        SAMPLE_RATE,
        VocalLevel::new(vocal_level).unwrap(),
        profile,
    );
    let latency = processor.latency_frames();
    let mut stream = input.to_vec();
    stream.extend(std::iter::repeat_n(0.0, (latency + 1_024) * 2));
    for chunk in stream.chunks_mut(512) {
        processor.process_stereo_interleaved(chunk);
    }
    (stream, latency)
}

fn settled_left_channel(stream: &[f32], latency: usize) -> Vec<f32> {
    let start = (latency + SETTLE_FRAMES) * 2;
    let end = (latency + FRAMES - SETTLE_FRAMES) * 2;
    stream[start..end]
        .chunks_exact(2)
        .map(|frame| frame[0])
        .collect()
}

fn settled_input_left(input: &[f32]) -> Vec<f32> {
    input[SETTLE_FRAMES * 2..(FRAMES - SETTLE_FRAMES) * 2]
        .chunks_exact(2)
        .map(|frame| frame[0])
        .collect()
}

#[test]
fn balanced_reduces_center_vocal_band_at_44100_hz() {
    let input = tone(1_000.0, false);
    let input_rms = rms(&settled_input_left(&input));
    let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
    let output_rms = rms(&settled_left_channel(&output, latency));
    assert!(
        output_rms / input_rms <= 0.40,
        "ratio={}",
        output_rms / input_rms
    );
}

#[test]
fn music_preservation_keeps_low_bass_at_44100_hz() {
    let input = tone(90.0, false);
    let input_rms = rms(&settled_input_left(&input));
    let (output, latency) = render(
        &input,
        0.0,
        ClassicSuppressionProfile::MusicPreservation,
    );
    let output_rms = rms(&settled_left_channel(&output, latency));
    assert!(
        output_rms / input_rms >= 0.70,
        "ratio={}",
        output_rms / input_rms
    );
}

#[test]
fn side_only_signal_and_latency_remain_safe_at_44100_hz() {
    let input = tone(1_000.0, true);
    let input_rms = rms(&settled_input_left(&input));
    let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
    let output_rms = rms(&settled_left_channel(&output, latency));
    let ratio = output_rms / input_rms;
    assert!((0.944..=1.059).contains(&ratio), "ratio={ratio}");
    assert!(latency <= (SAMPLE_RATE as usize * 25 / 1_000));
    assert!(output.iter().all(|sample| sample.is_finite()));
}
